
System Correctness: Confidence
==============================

In the frame of the Byzantine Generals problem, the correctness of a
distributed coordination system is analyzed through the lens of "fault
tolerance". In our frame we take on a broader scope and address the
question of the many kinds of confidence necessary for a system's adoption
and continued use. We identify and address the following dimensions
necessary confidence:

1.  **Fault Tolerance:** the system's resilience to external
  perturbations, both malicious and natural. Intrinsic integrity.

2.  **Completeness/Fit:** the system's apriori design elements that
  demonstrate fitness for purpose. We demonstrate this by describing
  how Holochain addresses: multi-agent reality binding, scalability,
  and shared-state finality.

3.  **Security:** the system's resilience to intentional disruption by
  malicious action.

4.  **Evolvability:** the system's inherent architectural affordances
  for increasing confidence over time, especially based on data from
  failures of confidence in the above dimensions.

Our claim is that if all of these dimensions are sufficiently addressed,
then the system takes on the properties of anti-fragility, that is, it
becomes more resilient and coherent in the presence of perturbations
rather than less.

Fault Tolerance
---------------

In distributed systems much has been written about Fault Tolerance
especially to those faults known as "Byzantine" faults. These faults
might be caused by either random chance or by malicious action. For
aspects of failures in system confidence that arise purely from
malicious action, see the [section on Security](#security).

1.  **Faults from unknown data provenance:** Because all data
  transmitted in the system is generated by Agents and is
  cryptographically signed by them, and those signatures are also
  included in the hash-chains, it is always possible to verify any
  datum's provenance. Thus, faults from intentional or accidental
  imposters, is not possible. Not that the system can prevent
  malicious or incautious actors from stealing or revealing private
  keys, however, the system does include affordances to deal with
  these realities. These are discussed under Completeness/Fit.

2.  **Faults from data corruptibility in transmission and storage:**
  Because all state data is stored along with a cryptographic hash
  of that data, and because all data is addressed and retrieved by
  that hash and can be compared against the retrieved data, the only
  possible fault is that the corruption resulted in data that has
  the same hash. For Sha256 hashing (which is what we use), this is
  known to be a vanishingly small possibility for both intentional
  and unintentional data corruption.[^corruption] Furthermore because all
  data is stored as hash-chains, it is not possible for portions of
  data to be retroactively changed. Agent's Source Chains thus
  become immutable append-only event logs.\
  \
  One possible malicious act that an Agent can take is to roll back
  their chain to some point and start publishing different data from
  that point forward. But, because the publishing protocol requires
  Agents to publish Actions to the neighborhood of their own public
  key, any actions that lead to a forked chain will be easily and
  immediately detected by simple matching more than one action
  linked to a previous action.\
  It is also possible to unintentionally rollback one's chain.
  Imagine a setting where a hard-drive corruption leads to a restore
  from a non-up-to-date backup. If a user starts adding to their
  chain from that state, it will appear as a rollback to
  validators.\
  \
  Holochain adds an affordance for such situations in which a
  good-faith actor can add a Record repudiating such an
  unintentional chain fork.

3.  **Faults from temporal indeterminacy:** In general these faults do
  not apply to the system described here because it only relies on
  temporality where it is known that one can rely on it, i.e., when
  recording Actions that take place locally as experienced by an
  Agent. As these temporally recorded Actions are shared into the
  space in which nodes may receive messages in an unpredictable
  order, the system still guarantees eventual consistency (though
  not uniform global state) because of the intrinsic integrity of
  recorded hash-chain, and the deterministic validation.
  Additionally see the section on "Entwined multi-agent state
  change" for more details on how some of the use cases addressed by
  consensus systems are handled in this system.


[^corruption]: CITATION NEEDED

Completeness/Fit
----------------

1.  **Multi-agent reality binding (Countersigning)**

The addition of the single feature of Countersigning to Holochain
enables our eventually consistent framework to provide most of the
consensus assurances people seek from decentralized systems.
Countersigning provides the capacity for specific groups of agents to
mutually sign a single state-change on all their respective
source-chains. It makes the deterministic validity of a single Entry
require the cryptographic signatures of multiple agents instead of just
one. Furthermore any slow-downs necessary to add coordinated
countersigned entries are not just localized to the parties involved,
they are also localized to just the DNAs involved. The same parties can
continue to interact on other DNAs.

The following are common use cases for countersigning, for a detailed
technical specification, please see the [Countersigning Spec (Appendix B)](hwp_B_countersigning_spec.md).

a.  **Multi-Agent State Changes:** Some applications require changes
  that affect multiple agents simultaneously. Consider the transfer
  of a deed or tracking a chain of custody, where Alice transfers
  ownership or custody of something to Bob and we want to produce an
  **atomic change across both of their source chains**. We must be
  able to prevent indeterminate states like Alice committing a
  change releasing an item without Bob having taken possession yet,
  or Bob committing an entry acknowledging possession while Alice's
  release fails to commit. Holochain provides a countersigning
  process for multiple agents to momentarily lock their chains while
  they negotiate one matching entry that each one commits to their
  chain. An entry which has roles for multiple signers requires
  signed chain headers from each counterparty to enter the
  validation process. This ensures no party's state changes unless
  every party's state changes.

b.  **Cryptocurrencies Based on P2P Accounting:** Extending the previous
  example, if Alice wants to transfer 100 units of a currency to
  Bob, they can both sign a single entry where Alice is in the
  spender role, and Bob the receiver. This provides similar
  guarantees as familiar double-entry accounting, ensuring changes
  happen to both accounts simultaneously. You can easily compute
  someone's balance by a replay of the transactions on their source
  chain, and you can hold both signing parties accountable for any
  fraudulent transfers breaking the data integrity rules of the
  currency application. There's no need for global time of
  transactions when each is clearly ordered by its sequence in the
  chains of the only accounts affected by the change.

c.  **Witnessed Authoritative Sequence:** Some applications may require
  an authoritative sequence of changes to a specific data type.
  Consider changes to membership of a group of administrators, where
  Carol and David are both members of the group, and Carol commits a
  change which removes David, and David commits a change which
  removes Carol from the group. With no global time clock to trust,
  who's change wins? An application can set up a small pool of
  witnesses and configure their countersigning session to require M
  50% of N) and whichever action the
  witnesses sign first would prevent the other action from getting
  signed, because either Carol or David would have been successfully
  removed and would no longer be authorized to remove the other.

d.  **Exclusive Control of Rivalrous Data:** Another common need for an
  authoritative time sequence involves determining control of
  rivalrous data such as name registrations. Using M of N signing
  from a witness pool makes it easy to require witnessing for only
  rivalrous data types, and forgo the overhead of witnessing for all
  other data. For example, a Twitter-like app would not need
  witnessing for tweets, follows, unfollows, likes, replies, etc,
  only for registration of new usernames and for name changes. This
  preserves the freedom for low-overhead and easy scaling by not
  forcing consensus to be managed on non-rivalrous data (which
  typically comprises the majority of the data in web apps).

e.  **Generalized Micro-Consensus Consent: Entwined multi-agent state
  change:** Even though Holochain is agent-centric and designed to
  make only local state changes, the countersigning process may be
  seen as an implementation of Byzantine consensus applied to
  specific data elements or situations. Contextual countersigning is
  exactly what circumvents the need for global consensus in
  Holochain applications.

```{=html}
<!-- -->
```
1.  **Scaling:** Holochain's architecture is specifically designed to maintain resilience and performance as both the number of users and interactions increase. Key factors contributing to its scaling capabilities include:
  a. Agent-centric approach: Unlike traditional blockchain systems, which require global consensus before progressing, Holochain adopts an agent-centric approach where changes made to an agent’s state become authoritative once signed to their chain and communicated to others via the DHT. As a result, agents are able to initiate actions without delay.
  b. Bottle-neck Free Sharded DHT: Holochain's DHT is sharded, meaning that each node only stores a fraction of the total data, reducing the storage and computational requirements for each participant. At the same time, the storage of content with agents whose public key is “near” the hash of each Action or Entry enables in combination with the use of Linking metadata to make the DHT a graphing DHT enables participants to create paths to quickly locate relevant content. When the agents responsible for validating a particular state change receive an authoring agent’s proposed state change, they are able to a) request information from others in the DHT regarding the prior state of the authoring agent (where relevant), and b) make use of their own copy of the apps validation rules to deterministically validate the change.
  While that agent and its validating peers are engaged with the creation and validation of a particular change to the state of the authors chain, in parallel, other agents are able to author state changes to their own chain and have these validated by the validating peers for each of those changes.  This bottle-neck free architecture allows users to continue interacting with the system without waiting for global agreement.
  With singular actions by any particular agent (and the validation of those actions by a small number of other agents) able to occur simultaneous with singular actions by other agents as well as countersigned actions by particular groups of agents. The network is not updating state globally (as blockchains typically do) but is instead creating, validating, storing and serving changes of the state of particular agents in parallel. 
  c. Multiple networks: In Holochain, each application (DNA) operates on its own independent network, effectively isolating the performance of individual apps. This prevents a high-traffic, data heavy, or processing heavy app from affecting the performance of other lighter apps within the ecosystem. Participants are able to decide for themselves which applications they want to participate in.
  \[TODO: ACB REVIEW could we add in any O(n) notation here?\] 
  d. Order of Complexity: "Big O" notation is usually only applied to local computation based on handling `n` number of inputs. However, if we consider a new type of O-notation for decentralized systems which includes two inputs `n` as the number transactions/inputs/actions, and `m` as the number of nodes/peers/agents/users. Most blockchain's are some variant of $O\ n^2*m$ in their order of complexity. Every node must gossip all state changes and perform all validate them all. However, Holochain retains $O\ \frac{log(n)}{m}$ complexity because of sharding storage and validation, as the number of nodes in the network grows, each node performs a smaller portion of the workload.

1.  **Shared-state Finality:** Many blockchains approximate chain
  finality by assuming that the "longest-chain wins." That strategy
  does not translate well to agent-centric chains which are simply
  histories of an agent's actions. While there is no concern about
  forking global state because a Holochain app doesn't have one, we
  can imagine a situation where Alice and Bob have countersigned a
  transaction, and then Alice forks her source chain by later
  committing an entry to an earlier sequence position in her chain.
  If she also set back her clock to make it look like Bob
  participated in a fork instead of her, this could break the
  cross-chain atomic change he was relying on. This can even happen
  non-maliciously when someone suffers data loss and restores from a
  backup after having made changes that did not get included in the
  backup. While the initial beta version of Holochain does not offer
  fork finality protections for source chains, later versions will
  incorporate "meta-data hardening" which enables gossipping peers
  to tentatively solidify a state of affairs when they see gossip
  for a time window has calmed and neighbors have converged on the
  same state. After this settling period (which might get set to
  something between 5 to 15 minutes) any later changes which would
  produce a conflict (such as forking a chain) can be rejected.

Security
--------

The system's resilience to intentional gaming, and disruption by
malicious actors will be covered in depth in future papers, but here we
provide an overview.

There are many contributing factors to what allows systems to live up to
the varying safety and security requirements of their users. In general,
the approach taken in Holochain is to provide affordances that take into
account the many types of real-world costs that result from adding
security and safety to systems such that application developers can
match the trade-offs of those costs to their application context. The
integrity guarantees listed in the formal system description, detail the
fundamental data safety that Holochain applications provide. Some other
important facets of system security and safety come from:

1.  Gating access functions that change state, for which Holochain
  provides a unified and flexible Object Capabilities model

2.  Detecting and Blocking participation of bad actors, for which
  Holochain provides the affordances of validation and warranting.

3.  Protection from Attack categories

4.  Resilience to Human Error

### Gating Access via Cryptographic Object Capabilities

To use a Holochain application end-users must trigger zome calls that
affect local state changes on their Source Chains. Additionally zome
functions can make calls to other zome functions on remote nodes in the
same app, or to other DNAs running on the same conductor. All of these
calls must happen in the context of some kind of permissioning system.
Holochain\'s security model for calls is based on the
Object-capability[^object_capability] security model, but for a distributed
cryptographic context in which we use cryptographic signatures to prove
the necessary agency for taking action.

[^object_capability]: https://en.wikipedia.org/wiki/Object-capability\_model

Access is thus mediated by Capability grants of 4 types:

-   author: only the agent owning the source change can make the zome
  call

-   assigned: only the named agent(s) can make the zome call

-   transferrable: anybody with the given token can make the zome call

-   unrestricted: anybody can make the zome call (no secret nor proof of
  authorized key needed to use this capability)

All zome calls must be signed and also take a required capability claim
parameter that MUST be checked by the system for making the call. Agents
record capability grants on their source chains and distribute them as
necessary according to the application\'s needs. Receivers of grants can
record them as claims (usually as a private entry) on their chains for
later lookup. The \"agent\" type grant is just the agent\'s public key.

### Validation & Warranting

We have already covered how Holochain's agent-centric validation and
intrinsic data integrity provides security from malicious actors trying
to introduce invalid or incorrect information into an Application's
network as every agent can deterministically verify data and thus
secure itself. It is also important, however, to be able to eject
malicious actors from network participation who generate or propagate invalid data so as to secure against the resource drain that such actions may incur. 

As agents publish their actions to the DHT, other agents serve as validators. When validation passes, they send a validation receipt back to the authoring agent, so they know the network has seen and stored their data. When validation fails, they send a negative validation receipt, known as a warrant, back to the author and their neighbors so the system can propagate these provably invalid attempted
actions. This also flags the offending agent as corrupted or malicious so that other nodes can block them and stop interacting with the offending agent. Every node can confirm the warrant for themselves as they are based on the shared deterministic validation rules, which all agents have a copy of.  

This enables a dynamic where any single honest agent can detect and report any invalid actions. So instead of needing a majority consensus to establish reliability of data (an "N/2 of N" trust model), Holochain enables "one good apple to heal the bunch" with a "1 of N" trust model for any data you acquire from agents on the network. 

For even stricter situations, apps can achieve a "0 of N" trust model, where no external agents need to be trusted, because nodes can always validate data for themselves, independent of what any other nodes say.

### Security from Attack Categories

#### Consensus​ ​Attacks

This whole category of attack starts from the assumption that consensus
is required for distributed systems. Because Holochain doesn't start
from that assumption the attack category really doesn't apply, but it's
worth mentioning because there​ ​are​ ​a​ ​number​ ​of​ ​attacks​ ​on​
​blockchain​ ​which​ ​disrupt their distributed computing solution through
collusion between some majority of nodes. ​The​ ​usual
thinking​ ​is​ ​that​ ​it​ ​takes​ ​a​ ​large​ ​number​ ​of​ ​nodes 
​and​ ​massive​ ​amounts​ ​of​ ​computing​ ​power or financial
incentives46​ ​to prevent​ ​undue​ ​hijacking​ ​of​ ​consensus.​
​However,​ ​since​ ​Holochain's data coherence doesn't derive from all
nodes proceeding in consensus​ lockstep,​ ​but​ rather ​on​ determinist
validation, nobody​ ​ever​ ​needs​ ​to​ ​trust​ ​a​ ​consensus​
​lottery.​ ​

#### Sybil Attacks

Since Holochain does not rely on any kind of majority consensus, it is already less vulnerable to Sybil Attacks, the creation of lots of fake colluding accounts which are typically used to overwhelm consensus of honest agents. But since Holochain enables "1 of N" and even "0 of N" trust models, Sybils simply cannot overwhem or disrupt honest agents.

Also, since Holochain is not a monolithic environment where every app and transaction run on a single chain in a single network, a Sybil Attack can only be attempted on a single app's network at a time. Also, unlike the absolutes of public vs. blockchains, each hApp can define their own membrane on a spectrum from very open and permissive to closed and strict by the kind of membership proof involved in passing into their network's membrane.

Membership proofs are passed in during the installation process of any Holochain app, so that it can be committed to the agent's chain just ahead of their public key. An agent's public key acts as their address in that applications DHT network, and is created during the genesis process in order to join the network. Other agents can confirm whether an agent's key can join by validating the membership proof.

A large variety of membership proofs are possible, ranging from none at all, loose social triangulation, or an invitation from any current user, to stricter invitation lists, proof of work requirements in generating your keys, or a kind of proof of stake showing you have deposited some value which you lose if your account gets warranted. 

We generally suggest that applications may want to enforce some kind of membrane against Sybils, not because consensus or data integrity is at risk but because carrying a lot of Sybils just makes unnecessary work for honest agents running an application. We cover more about this in the next section.

#### Spamming Attacks

Holochain includes a native rate-limiting on entry creation \[TODO:
ACB\]


\[Arthur thinks this next DPKI stuff is out of scope for Sybil Attacks\]

\[expand on: Holochain enables​ ​continuity​ ​of​ ​identity​ ​across​
​application​ ​contexts​ ​with​ ​its​ ​DPKI​ ​app,​ ​which​ ​can​
​interface with​ ​decentralized​ ​identity​ ​services​ ​of​ ​your​
​choosing.\]

#### Denial-of-Service Attacks

Holochain is not systemically subject to denial-of-service attacks
because there is no central point to attack. Because each application is
its own network, attackers would have to flood every agent of every
application to carry out a systemic denial-of-service attack and to do
what would require knowing who all those agents are which is also not
recorded anywhere. One point of vulnerability are the boot-strap servers
for an application. But this is not a systemic vulnerability as each
application can designate its own boot-strap server, and they can also
be arbitrarily hardened against denial-of-service to suit the needs of
the application.



#### Eclipse Attacks

This is a standard vulnerability for DHTs. How do you know you are
talking to honest nodes so that you are getting an honest picture of the
network. If all the nodes you are talking to are falsifying data...
(then what?) If the first node you talk to is malicious you may never
get any honest node. We are proposing the following solution: \[TODO:
ACB\]

### Human​ ​Error

There are some aspects of security, especially those of human error,
that all systems are subject to. People​ ​will​ ​still​ ​lose​ ​their​
​keys,​ ​use​ ​weak​ ​passwords,​ ​get​ ​computer​ ​viruses, etc.​ ​​
But, crucially, in the realm of "System Correctness" and "confidence,"​
the question that needs addressing is how the system interfaces with
mechanisms to mitigate against human error. Holochain provides
significant tooling to support key management in the form of its ​core​
​Distributed​ ​Public Key​ ​Infrastructure (DPKI) and DeepKey app built
on that infrastructure. Among other things, this tooling ​provides​
​assistance​ ​in​ ​managing​ ​keys,​ ​managing​ ​revocation​ ​methods,​
​and reclaiming​ ​control​ ​of​ ​applications​ ​when​ ​keys​ ​or​
​devices​ ​have​ ​become​ ​compromised. \[TODO: ACB\] \[Need to be able
to refer to external docs on DeepKey and DPKI\]

Evolvability
------------

For large scale systems to work well over time, we contend that specific
architectural elements and affordances make a significant difference in
their capacity to evolve while maintaining overall coherence as they do
so:

1.  **Subsidiarity:** From the Wikipedia definition: "*Subsidiarity is a
  principle of social organization that holds that social and
  political issues should be dealt with at the most immediate (or
  local) level that is consistent with their resolution.*"
  Subsidiarity enhances evolvability because it insulates the whole
  system from too much change, while simultaneously allowing change
  where it is needed. Architecturally, however, subsidiarity is not
  easy to implement because it is rarely immediately obvious what
  level of any system is consistent with an issue's resolution.\
  \
  In Holochain, the principle of subsidiarity is embodied many ways,
  but crucially in the architecture of app instances having fully
  separate DNAs running on their own separate networks, each also
  having clear and differentiable Integrity and Coordination
  specifications. This creates a very clear loci of change, both at
  the level of when the integrity rules of a DNA need to change, and
  at the level of how one interacts with a DNA. This allows
  applications to evolve exactly in the necessary area by updating
  only the DNA and DNA portion necessary for changing the specific
  functionality that needs evolving.

2.  **Grammatic composability:** Highly evolvable systems are built of
  grammatic elements that compose well with each other both
  "horizontally", which is the building of a vocabulary that fills
  out a given grammar, and "vertically" which is creating new
  grammars out of expressions of a lower level grammar. There is
  much more that can be said about grammatics and evolvability, but
  that is out-of-scope for this paper. However, we contend that the
  system as described above lives up to these criteria of having
  powerful grammatical elements that compose well as described. DNAs
  are essentially API definitions that can be used to create a large
  array of micro-services that can be assembled into small
  applications. Applications themselves can be assembled at the User
  Interface level. A number of frameworks in the Holochain ecosystem
  are already building off of this deep level capacity for
  evolvability that's built into the system's architecture[^evolvability].

3.  **Membranics:** \[todo: EHB\]

[^evolvability]: We, Neighborhoods, Ad4m (https://ad4m.dev/) \[TODO: insert links
    here\]