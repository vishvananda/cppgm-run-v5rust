// N3485 focus: 12.4 [class.dtor], 14.7.1 [temp.inst]
// Declaring an owning destructor does not instantiate a member-specialization
// destructor before the private implementation type is completed.  Defining
// the owning destructor after completion may then demand the valid body.

template<class T>
struct holder
{
  ~holder()
  {
    (void)sizeof(T);
  }
};

class owner
{
  struct implementation;
  holder<implementation> value;

public:
  owner();
  ~owner();
};

struct owner::implementation
{
};

owner::owner()
{
}

owner::~owner()
{
}

int main()
{
  owner value;
  (void)value;
  return 0;
}
