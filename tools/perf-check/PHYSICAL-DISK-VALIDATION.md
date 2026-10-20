# Physical-storage validation plan (not executed)

The Development workload is not physical redundancy evidence. Execute this plan only in an explicitly approved disposable lab. No destructive-device commands are included or run by the harness.

## Preconditions and authorization

1. Obtain the owner's approval for each dedicated test device, system volume, controller and fault-injection method. Record stable serial/WWN, physical topology, mountpoints, capacity, firmware, filesystem and power-loss-protection/cache settings. Confirm no production/customer data, backups or unrelated mounts use them.
2. Use three genuinely independent physical member devices plus an independently managed system volume. Three partitions, loopbacks, directories or virtual disks on one physical drive do not qualify. Record shared power/controller failure domains explicitly.
3. Back up anything required outside the approved disposable equipment. Provide a supervised stop/recovery plan. Verify XCS production identity checks accept independent devices and reject aliases/duplicate physical devices. Initialization itself requires separate approval; do not reuse the Development fixtures as production evidence.
4. Pin repository revision, Rust/build profile, OS/kernel/filesystem versions and workload seed. Capture logs with secrets removed. Record successful client acknowledgments externally before faults.

## Fault matrix

For every supported replica policy and relevant placement topology:

- Establish acknowledged versions, hashes, metadata sequence and independent offline-export oracle.
- Remove one approved member through a lab-supported hot-unplug mechanism. Verify surviving acknowledged content, explicit degraded health and policy-appropriate write rejection or behavior. Never infer quorum rules from the number of directories.
- Restore/reintroduce that member using supported recovery. Verify stale members cannot override newer acknowledged metadata; repair/replication must preserve hashes and policy.
- Interrupt the application process at each publication fault-injection boundary. Restart and require exactly the published operation outcome or a safe explicit failure; retries must not create conflicting acknowledged versions.
- Separately test abrupt system power loss on the approved rig, including cache behavior. A process kill does not emulate loss of device/controller write cache. Record which writes were externally acknowledged before interruption.
- Test system-volume loss using the documented restore flow and sufficient surviving trusted metadata members. Export and compare all acknowledged versions from surviving replicas.
- Test loss of more devices than the configured tolerance: require explicit unavailability/quorum errors rather than corrupted bytes, forged success or silent new initialization.
- On approved disposable copies only, introduce controlled payload and metadata corruption. Require checksum rejection, verified replica fallback and no publication based solely on an untrusted index.
- Test long reads concurrent with saves, repair and supported purge operations. Validate range checksums and leases, memory bounds and writer lock latency through the actual HTTP path.

## Acceptance and evidence

Retain revision/configuration, device independence evidence, exact fault timing, acknowledged operation IDs, engine and offline-check results, full export hashes, recovery duration, rejected operations and unexplained deviations. A pass requires every acknowledged retained version recoverable within the advertised fault tolerance, no unverified corrupt bytes, no stale metadata winning recovery and correct explicit failures outside tolerance. Publish performance percentiles only with repeated controlled runs and confidence intervals. Label process-crash, logical-corruption and real power-loss evidence separately.
