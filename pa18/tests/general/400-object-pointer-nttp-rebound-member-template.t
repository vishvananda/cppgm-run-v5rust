template<bool *P, class Entry, class Value>
struct factory_class {
  factory_class()
  {
    *P = true;
  }
};

template<bool *P>
struct factory_marker {
  template<class Entry, class Value>
  struct apply {
    typedef factory_class<P, Entry, Value> type;
  };
};

namespace ns {
bool mark = false;

template<class FactorySpecifier>
struct core {
  typedef typename FactorySpecifier::template apply<int, int>::type factory_type;

  struct holder_arg {
    factory_type factory;
  };

  static bool init()
  {
    holder_arg a;
    (void)a.factory;
    return true;
  }
};

bool init = core<factory_marker<&mark> >::init();
}

int main()
{
  return ns::mark ? 0 : 1;
}
