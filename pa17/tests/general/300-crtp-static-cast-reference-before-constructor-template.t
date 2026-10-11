// VALIDATION: compile-pass

struct init_seq
{
};

template<typename T>
struct iterator_base
{
  T const & cast() const
  {
    return static_cast<T const &>(*this);
  }
};

struct payload
{
  payload() {}
  payload(init_seq const &) {}
};

struct iterator : iterator_base<iterator>
{
  template<typename Init>
  iterator(Init const & init)
    : value(init)
  {
  }

  payload value;
};

iterator const begin(init_seq & seq)
{
  return iterator(seq);
}

int deref(iterator const &)
{
  return 7;
}

template<typename It>
int operator*(iterator_base<It> const & it)
{
  return deref(it.cast());
}

int front(init_seq & seq)
{
  return *begin(seq);
}

int main()
{
  init_seq seq;
  return front(seq) == 7 ? 0 : 1;
}
