typedef unsigned long size_t;

template<class T, T V>
struct constant
{
  static const T value = V;
};

template<class...>
struct list {};

struct A { static const size_t value = 8; };
struct B { static const size_t value = 4; };

template<size_t Len, size_t A1, size_t A2>
struct select
{
  static const size_t value = Len < A1 ? A2 : A1;
};

template<class List, size_t Len>
struct find;

template<class Head, size_t Len>
struct find<list<Head>, Len> : constant<size_t, Head::value> {};

template<class Head, class... Tail, size_t Len>
struct find<list<Head, Tail...>, Len>
    : constant<size_t,
               select<Len, Head::value, find<list<Tail...>, Len>::value>::value> {};

int main()
{
  return find<list<A, B>, 6>::value == 4 ? 0 : 1;
}
