template <class T> struct iterator_traits;
template <class T> struct iterator_traits<T*> { typedef long difference_type; };

template <class I> using diff_t = typename iterator_traits<I>::difference_type;

struct tag {};

template <class P, class I>
int f(I, diff_t<I>) { return 7; }

int main() {
  return f<tag>((const char*)0, 1);
}
