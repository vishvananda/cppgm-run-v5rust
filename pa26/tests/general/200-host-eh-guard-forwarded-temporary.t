// N3485 15.5.1p1, 12.2p3, 12.4p3: an exception leaving a noexcept function, or
// a destructor, calls std::terminate; one caught inside it does not, and the
// full-expression's temporaries are destroyed when it completes
