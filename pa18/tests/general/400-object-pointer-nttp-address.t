namespace boost_like {
bool mark = false;

template<bool *P>
struct marker {
  marker()
  {
    *P = true;
  }
};

marker<&mark> instantiated;
}

template<bool *P>
struct writer {
  static void set()
  {
    *P = true;
  }
};

template<const bool *P>
struct reader {
  static int get()
  {
    return *P ? 0 : 1;
  }
};

int main()
{
  boost_like::mark = false;
  writer<&boost_like::mark>::set();
  return reader<&boost_like::mark>::get();
}
