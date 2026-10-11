namespace std {
inline namespace __1 {
template<class T>
T min(T a, T b)
{
  return a < b ? a : b;
}
}  // namespace __1
}  // namespace std

template<class T>
struct traits {
  typedef T size_type;
};

template<class Graph>
struct C {
  typedef typename traits<Graph>::size_type size_type;

  size_type f(size_type a, size_type b)
  {
    using std::min;
    size_type c = min(a, b);
    return c;
  }
};

int main()
{
  C<int> c;
  return c.f(2, 1);
}
