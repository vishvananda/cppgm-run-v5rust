typedef decltype(sizeof(0)) size_t;

template <class Size, size_t = sizeof(Size) * 8>
struct H;

template <class Size>
struct H<Size, 32> {
  Size f(Size len) const {
    switch (len) {
    case 3:
      [[__fallthrough__]];
    case 2:
      [[__fallthrough__]];
    case 1:
      return len;
    }
    return len;
  }
};
