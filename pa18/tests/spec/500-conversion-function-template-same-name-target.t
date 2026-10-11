// VALIDATION: compile-pass
// N3485 focus: 12.3.2 [class.conv.fct], 14.8.2.3 [temp.deduct.conv]

namespace target_ns {
  template<class T>
  struct view {
    int value;

    view()
      : value(0)
    {
    }

    explicit view(int v)
      : value(v)
    {
    }
  };
}

namespace source_ns {
  template<class T>
  struct view {
    int value;

    explicit view(int v)
      : value(v)
    {
    }

    template<class U>
    operator target_ns::view<U>() const
    {
      return target_ns::view<U>(value + 1);
    }
  };
}

int consume(target_ns::view<char> v)
{
  return v.value;
}

int main()
{
  source_ns::view<char> v(6);
  return consume(v) == 7 ? 0 : 1;
}
