// N3485 5.1.2p5, 15.4p9, 15.5.1: an exception leaving a lambda declared
// noexcept calls std::terminate() (not caught by the caller's caller)
