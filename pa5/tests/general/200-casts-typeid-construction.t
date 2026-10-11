struct C {};
template<class T>
void casts(const C* pointer) {
  const_cast<C*>(pointer);
  reinterpret_cast<const char*>(pointer);
  typename T::value(123);
  C{};
}
