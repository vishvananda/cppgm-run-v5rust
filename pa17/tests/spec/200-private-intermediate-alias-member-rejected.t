// VALIDATION: compile-fail
// N3485 focus: 11 [class.access], 14.6 [temp.res]

struct result_owner
{
  typedef int type;
};

template<class T>
struct wrapper
{
private:
  typedef result_owner hidden;
};

typedef wrapper<int>::hidden::type rejected;

int main()
{
  return 0;
}
