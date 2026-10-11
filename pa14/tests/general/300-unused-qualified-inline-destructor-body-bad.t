struct BrokenDestructor
{
  ~BrokenDestructor();
};

inline BrokenDestructor::~BrokenDestructor()
{
  missing_destructor_name = 1;
}

int main()
{
  return 0;
}
