template<class T>
struct Holder {
  struct Location {
    int value;
  };

  Location locate() const;
  int read() const;
};

template<class T>
auto Holder<T>::locate() const -> Location
{
  Location loc;
  loc.value = 7;
  return loc;
}

template<class T>
int Holder<T>::read() const
{
  return locate().value;
}

int main()
{
  Holder<int> h;
  return h.read() == 7 ? 0 : 1;
}
