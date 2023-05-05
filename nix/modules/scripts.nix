{ self, lib, ... }: {
  perSystem = { config, self', inputs', pkgs, ... }: {
    # Definitions like this are entirely equivalent to the ones
    # you may have directly in flake.nix.
    packages = {
      scripts-ci-cachix-helper =
        let
          pathPrefix = lib.makeBinPath (with pkgs; [ cachix ]);
        in
        pkgs.writeShellScript "scripts-ci-cachix-helper" ''
            #! /usr/bin/env nix-shell
            set -euo pipefail

          export PATH=${pathPrefix}:$PATH

          export PATHS_PREBUILD_FILE="''${HOME}/.store-path-pre-build"

          case ''${1} in
            setup)
              if [[ -n ''${CACHIX_AUTH_TOKEN:-} ]]; then
                  echo Using CACHIX_AUTH_TOKEN
                  cachix --verbose authtoken ''${CACHIX_AUTH_TOKEN}
              fi
              cachix --verbose use -m user-nixconf ''${CACHIX_NAME:?}
              nix path-info --all > "''${PATHS_PREBUILD_FILE}"
              ;;

            push)
              comm -13 <(sort "''${PATHS_PREBUILD_FILE}" | grep -v '\.drv$') <(nix path-info --all | grep -v '\.drv$' | sort) | cachix --verbose push ''${CACHIX_NAME:?}
              ;;
          esac
        '';

      scripts-repo-flake-update = pkgs.writeShellScriptBin "scripts-repo-flake-update" ''
        set -xeuo pipefail
        trap "cd $PWD" EXIT

        # we want to denote relative directories with './' so that `nix flake` interprets it as a `git+file` source type which we prefer.
        # otherwise it would use `path`.
        export VERSIONS_DIR="./versions/''${1}"
        export DEFAULT_VERSIONS_DIR="./$(nix eval --impure --raw --expr '(builtins.fromJSON (builtins.readFile ./flake.lock)).nodes.versions.locked.dir')"

        (
          cd "$VERSIONS_DIR"
          nix flake update --tarball-ttl 0
        )

        if [[ $(${pkgs.git}/bin/git diff -- "$VERSIONS_DIR" | grep -E '^[+-]\s+"' | grep -v lastModified --count) -eq 0 ]]; then
          echo got no actual source changes, reverting modifications..
          ${pkgs.git}/bin/git checkout $VERSIONS_DIR/flake.lock
          exit 0
        fi

        git commit "$VERSIONS_DIR" -m "chore(flakes): update $VERSIONS_DIR"

        if [[ "$VERSIONS_DIR" != "$DEFAULT_VERSIONS_DIR" ]]; then
          exit 0
        fi

        echo default versions $VERSIONS_DIR updated, updating toplevel flake
        nix flake lock --tarball-ttl 0 --update-input versions --override-input versions "$VERSIONS_DIR"

        if [[ $(${pkgs.git}/bin/git diff -- flake.lock | grep -E '^[+-]\s+"' | grep -v lastModified --count) -eq 0 ]]; then
          echo got no actual source changes in the toplevel flake.lock, reverting modifications..
          ${pkgs.git}/bin/git checkout flake.lock
          exit 0
        fi


        # TODO: rewrite lock file to point to github
        head_rev=$(git rev-parse HEAD)
        nar_hash=$(nix hash path $(nix eval --impure --raw --expr "builtins.getFlake git+file://$PWD?rev=$(git rev-parse HEAD)"))
        nix eval --impure --json --expr "
          let
            lib = (import ${pkgs.path} {}).lib;
            lock = builtins.fromJSON (builtins.readFile ./flake.lock);
            lock_updated = lib.recursiveUpdateUntil 
              (path: l: r: path == [\"nodes\" \"versions\" \"locked\"]) 
              lock 
              {
                nodes.versions.locked = {
                  dir = \"versions/''${1}\";
                  type = \"github\";
                  owner = \"holochain\";
                  repo = \"holochain\";
                  rev = \"$head_rev\";
                  narHash = \"$nar_hash\";
                };
              }
              ;
          in lock_updated
        " | jq --raw-output . > flake.lock.new
        mv flake.lock{.new,}

        git commit flake.lock -m "chore(flakes): update $VERSIONS_DIR"
      '';

      scripts-release-automation-check-and-bump = pkgs.writeShellScriptBin "scripts-release-automation-check-and-bump" ''
        set -xeuo pipefail

        ${self'.packages.release-automation}/bin/release-automation \
            --workspace-path=$PWD \
            --log-level=debug \
            crate detect-missing-releaseheadings

        ${self'.packages.release-automation}/bin/release-automation \
          --workspace-path=''${1} \
          --log-level=debug \
          --match-filter="^(holochain|holochain_cli|kitsune_p2p_proxy)$" \
          release \
            --no-verify \
            --force-tag-creation \
            --force-branch-creation \
            --additional-manifests="crates/test_utils/wasm/wasm_workspace/Cargo.toml" \
            --allowed-semver-increment-modes="!pre_minor beta-dev" \
            --steps=CreateReleaseBranch,BumpReleaseVersions

        ${self'.packages.release-automation}/bin/release-automation \
            --workspace-path=''${1} \
            --log-level=debug \
            release \
              --dry-run \
              --no-verify \
              --steps=PublishToCratesIo
      '';

      scripts-ci-generate-readmes = pkgs.writeShellScriptBin "scripts-ci-generate-readmes" ''
        crates_to_document=("hdi" "hdk" "holochain_keystore" "holochain_state")

        for crate in "''${crates_to_document[@]}"; do
            echo 'generating README for crate' "$crate"
            ${self'.packages.cargo-rdme} -w $crate --intralinks-strip-links --force
        done

        # have any READMEs been updated?
        git diff --exit-code --quiet
        readmes_updated=$?
        if [[ "$readmes_updated" == 1 ]]; then
            echo 'READMEs have been updated, committing changes'
            git config --local user.name release-ci
            git config --local user.email ci@holo.host
            git commit -am "docs(crate-level): generate readmes from doc comments"
            git config --local --unset user.name
            git config --local --unset user.email
        fi
      '';
    };

  };
}
