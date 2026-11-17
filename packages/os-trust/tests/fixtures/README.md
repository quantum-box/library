Public certificate fixtures from <https://emudhra.we-have-a-problem.com/>
(`roots.json` and `chains.zip`, captured for the distrust test). The
`.crt` extension keeps them out of the repository's `*.pem` ignore rule,
which exists so private keys are not committed.

These are the compromised roots and the chains served by the incident's
test hosts. They are not trust anchors. The test plants a root into a
temporary hash directory to prove that directory is what makes OpenSSL
accept the chain, then proves the pinned store does not.
