struct context {};
struct first;
struct second;
template<class T> struct proxy;

template<class T>
int specialized(const context &, T &, int);

template<class T>
int specialized(const context &, T *, int, int);

template<class T>
int specialized(const context &, proxy<T> &, int);

template<class T>
int specialized(const context &, const proxy<T> &, int);

template<>
int specialized<first>(const context &, first &, int);

template<>
int specialized<const first>(const context &, const first &, int);

template<>
int specialized<second>(const context &, second &, int);

template<>
int specialized<const second>(const context &, const second &, int);

struct first {};
struct second {};
template<class T> struct proxy {};

template<class T>
int specialized(const context &, T &, int)
{
  return 0;
}
