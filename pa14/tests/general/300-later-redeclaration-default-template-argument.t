template<class T, class U>
struct box
{
  typedef U type;
};

template<class T>
struct fallback
{
  typedef T type;
};

template<class T, class U = fallback<T> >
struct box;

typedef box<int>::type result;

int expect(fallback<int> *);

int main()
{
  result * ptr = 0;
  return expect(ptr);
}
