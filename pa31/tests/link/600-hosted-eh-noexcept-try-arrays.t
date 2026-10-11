// N3485 15.3p2, 15.5.1p1: inside a noexcept function, or a destructor, an
// exception from an array element's construction or destruction, a
// new-expression's elements or a std::initializer_list's array is caught by an
// enclosing handler of the same function after the elements already
// constructed are destroyed; std::terminate is called only when no handler
// takes it
