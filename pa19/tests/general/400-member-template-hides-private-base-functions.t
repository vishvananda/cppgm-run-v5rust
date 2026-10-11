namespace meta {
struct true_ {};
struct false_ {};
}

struct traits {
  typedef char char_type;
};

struct private_base {
  bool test(char, traits const &, meta::false_) const { return false; }
  bool test(char, traits const &, meta::true_) const { return false; }
};

template <class Traits>
struct derived_set : private private_base {
  typedef typename Traits::char_type char_type;

  template <class ICase>
  bool test(char_type, Traits const &, ICase) const {
    return true;
  }
};

template <class Traits, class ICase, class CharSet>
struct matcher {
  bool match(char ch, Traits const &tr) const {
    return this->charset_.test(ch, tr, ICase());
  }

  CharSet charset_;
};

int main() {
  matcher<traits, meta::true_, derived_set<traits> > value;
  traits tr;
  return value.match('x', tr) ? 0 : 1;
}
