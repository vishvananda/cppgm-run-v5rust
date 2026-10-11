// Declarations the units share.
#pragma once
template<class T>
auto counter(T) -> decltype(new T);
template<class T>
auto counter(T) -> decltype(new T)
{
	static long n = 0;
	++n;
	return reinterpret_cast<T*>(n);
}
template<class T>
auto counter_init(T t) -> decltype(new T(t))
{
	static long n = 0;
	n += 2;
	return reinterpret_cast<T*>(n);
}
template<class T>
auto counter_global(T t) -> decltype(::new T{ t })
{
	static long n = 0;
	n += 3;
	return reinterpret_cast<T*>(n);
}
template<class T>
auto release(T* p) -> decltype(delete p)
{
	static int n = 0;
	++n;
	*p = n;
}
template<class T>
auto release_array(T* p) -> decltype(delete[] p)
{
	static int n = 0;
	n += 10;
	*p = n;
}
long a_side();
