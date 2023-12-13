use std::path::PathBuf;

use holochain::{
    conductor::{
        api::{error::ConductorApiError, AdminInterfaceApi, RealAdminInterfaceApi},
        error::ConductorError,
    },
    sweettest::{SweetConductor, SweetDnaFile},
    test_utils::inline_zomes::simple_create_read_zome,
};
pub use holochain_conductor_api::*;
use holochain_types::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn initialize_deepkey() {
    holochain_trace::test_run().ok();

    let mut conductor = SweetConductor::from_standard_config().await;
    let admin_api = RealAdminInterfaceApi::new(conductor.raw_handle());

    let (dna, _, _) =
        SweetDnaFile::unique_from_inline_zomes(("simple", simple_create_read_zome())).await;

    let fake_dna_hash = ::fixt::fixt!(DnaHash);
    let dna_fake_dpki =
        dna.update_modifiers(DnaModifiersOpt::none().with_dpki_hash(fake_dna_hash.clone()));

    let dna_dpki =
        dna.update_modifiers(DnaModifiersOpt::none().with_dpki_hash(fake_dna_hash.clone()));

    {
        // - App can't be installed without a DPKI instance, since it specifies a DPKI hash dependency
        let err = conductor
            .setup_app("fail", &[dna_fake_dpki.clone()])
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ConductorApiError::ConductorError(ConductorError::DpkiHashMismatch(None, h))
            if h == fake_dna_hash
        ));
    }

    // Initialize deepkey
    let dpki_hash = {
        let deepkey_dna =
            DnaBundle::read_from_file(&PathBuf::from("./tests/conductor_services/deepkey.dna"))
                .await
                .unwrap();
        let (deepkey_dna, _) = deepkey_dna.into_dna_file(Default::default()).await.unwrap();
        let dpki_hash = deepkey_dna.dna_hash().clone();
        let response = admin_api
            .handle_admin_request(AdminRequest::InitializeDeepkey { deepkey_dna })
            .await;
        dbg!(&response);
        assert!(matches!(response, AdminResponse::Ok));
        dpki_hash
    };

    assert!(conductor.services().dpki.is_some());

    // Install app
    {
        // - App will be installed even without a DPKI hash
        conductor.setup_app("no_dpki", &[dna]).await.unwrap();

        // - App won't be installed with the wrong DPKI hash
        let err = conductor
            .setup_app("fail", &[dna_fake_dpki.clone()])
            .await
            .unwrap_err();
        assert!(matches!(
        err,
            ConductorApiError::ConductorError(ConductorError::DpkiHashMismatch(s, h))
            if s == Some(dpki_hash) && h == fake_dna_hash
        ));

        // - App will be installed with the correct DPKI hash
        conductor
            .setup_app("installed_app_id", &[dna_dpki])
            .await
            .unwrap();
    }
}
