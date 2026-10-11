// A qualified function-template call can be the argument that decides an
// ordinary member overload set. The callee may live only in the template
// registry, not ordinary value lookup.
namespace iterlib {
struct input_iterator_tag {};
struct forward_iterator_tag : input_iterator_tag {};

struct iter {
  int pos;
};

int operator-(iter last, iter first)
{
  return last.pos - first.pos;
}

template<class Iter>
input_iterator_tag iterator_category(const Iter &)
{
  return input_iterator_tag();
}

struct range_sink {
  int size;

  range_sink() : size(0) {}

  void assign(iter first, iter last)
  {
    construct(first, last, iterlib::iterator_category(first));
  }

  void construct(iter first, iter last, input_iterator_tag)
  {
    size = last - first;
  }

  void construct(iter, iter, forward_iterator_tag)
  {
    size = 99;
  }
};
}

int main()
{
  iterlib::iter first = {0};
  iterlib::iter last = {3};
  iterlib::range_sink s;
  s.assign(first, last);
  return s.size == 3 ? 0 : 1;
}
