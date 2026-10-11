// N3485 15.3p4, 15.3p15, 15.5.1p1: a member initializer that throws in a
// noexcept constructor whose body is a function-try-block destroys the members
// already constructed and enters the handler; reaching the handler's end
// rethrows the exception, which leaves the noexcept function, so
// std::terminate is called
