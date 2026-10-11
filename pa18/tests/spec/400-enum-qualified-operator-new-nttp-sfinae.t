void* operator new(unsigned long);
enum E {};
template<class T, void* (*P)(unsigned long) = &T::operator new> char probe(int);
template<class> long probe(...);
static_assert(sizeof(probe<E>(0)) == sizeof(long), "");
