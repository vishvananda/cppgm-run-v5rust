// HHC-082
typedef unsigned long size_t;

template<class T, size_t N>
void swap(T (&a)[N], T (&b)[N]) noexcept;
