// expect reject
// Access exemption does not turn a private alias into a class template.
class Owner {
  template<class T> struct Box { T value; };
  template<class T> using Alias = Box<T>;
};
extern template struct Owner::Alias<int>;
