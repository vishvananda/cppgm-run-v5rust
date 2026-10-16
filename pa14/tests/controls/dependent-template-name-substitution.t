// expect exit 0
// A dependent template-id retains its qualifier and written arguments in the
// function signature, then uses the selected template's defaults on demand.
struct Source {
  template<class Element, class Result = int>
  struct rebind {
    typedef Result type;
    Element value;
  };

  template<class Element, class = typename Element::missing>
  struct invalid { typedef int type; };
};

template<class Owner>
typename Owner::template rebind<char>::type scalar(Owner);

template<class Renamed>
typename Renamed::template rebind<char>::type scalar(Renamed) {
  return 7;
}

template<class Owner>
typename Owner::template rebind<int> object(Owner) {
  return typename Owner::template rebind<int>{5};
}

template<class Owner>
typename Owner::template rebind<int, char, long>::type wrong_count(Owner);
int wrong_count(...) { return 3; }

template<class Owner>
typename Owner::template invalid<int>::type wrong_default(Owner);
int wrong_default(...) { return 4; }

int main() {
  Source source;
  return scalar(source) == 7 && scalar(source) == 7
      && object(source).value == 5 && wrong_count(source) == 3
      && wrong_default(source) == 4 ? 0 : 1;
}
