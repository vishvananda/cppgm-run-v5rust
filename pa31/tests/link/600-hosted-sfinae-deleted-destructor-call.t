// N3485 14.8.2/8 + 8.4.3/2 (20.9.4.3 is_destructible): decltype(p->~T())
// naming a deleted destructor is a substitution failure, so
// std::is_destructible<D> is false
