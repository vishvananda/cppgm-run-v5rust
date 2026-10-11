struct stream {};

namespace fusion {
  template<class... T>
  struct vector {};

  template<class... T>
  using vector0 = vector<T...>;

  namespace operators {
    template<class Sequence>
    stream& operator<<(stream& os, Sequence const& seq)
    {
      return os;
    }
  }

  using operators::operator<<;

  namespace result_of {
    template<class Seq>
    struct clear {
      typedef vector0<> type;
    };
  }

  template<class Seq>
  typename result_of::clear<Seq const>::type clear(Seq const& s)
  {
    return vector0<>();
  }
}

int main()
{
  stream os;
  fusion::vector<int> s;
  os << fusion::clear(s);
  return 0;
}
