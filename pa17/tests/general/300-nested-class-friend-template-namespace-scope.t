class Locale {
  class Impl;

  template<typename Facet>
  friend const Facet * try_use_facet(const Locale &);

  Impl * impl;
};

class Locale::Impl {
  template<typename Facet>
  friend const Facet * try_use_facet(const Locale &);

  int * facets;
};

struct Ctype {};

template<typename Facet>
const Facet * try_use_facet(const Locale & loc)
{
  int * facets = loc.impl->facets;
  (void)facets;
  return 0;
}

const Ctype * force(const Locale & loc)
{
  return try_use_facet<Ctype>(loc);
}

int main()
{
  return 0;
}
