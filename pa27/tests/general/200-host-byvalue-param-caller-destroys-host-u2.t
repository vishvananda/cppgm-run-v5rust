// host-object link: Itanium C++ ABI 3.1.1 (and N3485 5.2.2p4 caller-side
// lifetime): a by-value parameter of class type with a non-trivial destructor
// is destroyed by the caller, not the callee; cppgm++/host mixes must destroy
// it exactly once
