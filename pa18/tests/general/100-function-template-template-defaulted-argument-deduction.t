// VALIDATION: compile-pass
// N3485 focus: 14.8.2.1 [temp.deduct.call]

namespace ttp_default_arg
{

template<class T>
struct less
{
};

template<class U>
struct plus
{
};

template<class U>
struct section
{
};

template<class T, class Compare>
struct interval
{
};

template<class T>
struct allocator
{
};

template<class T, class U,
         class Traits,
         class Compare = less<T>,
         class Combine = plus<U>,
         class Section = section<U>,
         class Interval = interval<T, Compare>,
         template<class> class Alloc = allocator>
struct interval_map
{
  void clear()
  {
  }
};

template<class T, class U, class Traits,
         template<class _T, class _U,
                  class _Traits,
                  class Compare = less<_T>,
                  class Combine = plus<_U>,
                  class Section = section<_U>,
                  class Interval = interval<_T, Compare>,
                  template<class> class Alloc = allocator> class IntervalMap,
         class Sequence>
void pass(const Sequence &,
          IntervalMap<T, U, Traits,
                      less<T>,
                      plus<U>,
                      section<U>,
                      interval<T, less<T> >,
                      allocator> & destination)
{
  destination.clear();
}

}

using namespace ttp_default_arg;

template<class T, class U, class Traits,
         template<class _T, class _U,
                  class _Traits,
                  class Compare = less<_T>,
                  class Combine = plus<_U>,
                  class Section = section<_U>,
                  class Interval = interval<_T, Compare>,
                  template<class> class Alloc = allocator> class IntervalMap>
void test()
{
  int seq = 0;
  IntervalMap<T, U, Traits> map;
  pass(seq, map);
}

int main()
{
  test<int, int, int, interval_map>();
  return 0;
}
