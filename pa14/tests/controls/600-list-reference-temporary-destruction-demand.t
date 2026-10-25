// A list argument owns its reference temporary's destructor demand. The frozen
// list-argument path does not check destructor access. expect exit 0
int destroyed;
struct Tag { static void destroy() { ++destroyed; } };
template<class T> struct Box {
  Box() {}
  ~Box() { T::destroy(); }
};
template<class T> struct Hidden {
  Hidden() {}
private:
  ~Hidden() { T::destroy(); }
};
template<class T> struct Whole {
  Box<T> member;
  Whole() {}
  ~Whole() {}
};
void accept(const Box<Tag>&) {}
void accept_nested(const Whole<Tag>&) {}
void accept_hidden(const Hidden<Tag>&) {}
struct Unused {};
void accept_unused(const Box<Unused>&);
void accept_unused_nested(const Whole<Unused>&);
inline void unused() { accept_unused({}); accept_unused_nested({}); }
int main() {
  accept({});
  accept_hidden({});
  accept_nested({});
  return destroyed == 3 ? 0 : 1;
}
