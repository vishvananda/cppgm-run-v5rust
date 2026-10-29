// A defined destructor does instantiate its member cleanup body.
template<class T> struct Ptr {
  T* p;
  ~Ptr() { static_assert(sizeof(T) > 0, "complete"); delete p; }
};
struct Owner {
  struct Impl;
  Ptr<Impl> p;
  ~Owner() {}
};
void destroy(Owner* owner) { owner->~Owner(); }
