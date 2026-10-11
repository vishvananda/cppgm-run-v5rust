namespace std {
struct ios_base {
  virtual ~ios_base();
};

ios_base::~ios_base() {}
}

int main()
{
  std::ios_base value;
  return 0;
}
