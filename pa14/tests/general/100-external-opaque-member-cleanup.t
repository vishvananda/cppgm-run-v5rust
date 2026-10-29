// External special-member calls must not instantiate opaque member cleanup.
template<class T> struct Ptr {
  T* p;
  ~Ptr() { static_assert(sizeof(T) > 0, "complete"); delete p; }
};
struct Owner {
  struct Impl;
  Ptr<Impl> p;
  Owner();
  ~Owner();
};
void use_owner() { Owner owner; }
