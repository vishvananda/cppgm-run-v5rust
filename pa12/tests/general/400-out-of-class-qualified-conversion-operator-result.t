namespace ns {
struct target {};
target global;
}

struct source {
  operator ns::target const& () const;
};

source::operator ns::target const& () const
{
  return ns::global;
}

int main()
{
  source s;
  ns::target const& r = s.operator ns::target const&();
  return &r == &ns::global ? 0 : 1;
}
