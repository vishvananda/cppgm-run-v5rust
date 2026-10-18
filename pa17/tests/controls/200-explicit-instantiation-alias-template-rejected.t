// expect reject
// An alias template-id does not name a class template for explicit instantiation.
template<class T> struct Box { T value; };
template<class T> using Alias = Box<T>;
template struct Alias<int>;
