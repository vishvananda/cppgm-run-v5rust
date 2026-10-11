struct mode {
};

template<class T>
struct close_impl {
};

struct linked_streambuf {
  int value;

  linked_streambuf() : value(0) {}

  void close(mode which)
  {
    value = 1;
    close_impl(which);
  }

  void close_impl(mode)
  {
    value = 7;
  }
};

int main()
{
  mode m;
  linked_streambuf x;
  x.close(m);
  return x.value == 7 ? 0 : 1;
}
