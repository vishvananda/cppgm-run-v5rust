// VALIDATION: compile-pass
// N3485 focus: 14.6.2.1 [temp.dep.type], 14.5.1 [temp.class]

template<class Owner>
struct iter
{
  typedef Owner owner;
};

template<class Iter>
struct traits
{
  typedef typename Iter::owner::later type;
};

template<class Iter>
struct reverse
{
  typedef typename traits<Iter>::type type;
};

template<class T>
struct basic
{
  typedef iter<basic> iterator;
  typedef reverse<typename basic::iterator> reverse_iterator;
  typedef int later;
};

typedef basic<int> json;
typedef typename json::reverse_iterator::type result;

int main()
{
  result x = 0;
  return x;
}
