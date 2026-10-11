// VALIDATION: compile-pass
// N3485 focus: 14.6.2.1 [temp.dep.type], 14.5.2 [temp.mem]

template<class T>
struct node
{
};

template<class T>
struct node_traits
{
  typedef node<T> * pointer;
};

template<class T, class P, class D>
struct tree_iter
{
};

template<class I>
struct map_iter
{
};

template<class T>
struct tree
{
  typedef node_traits<T> __node_traits;
  typedef typename __node_traits::pointer __node_pointer;
  typedef int difference_type;
  typedef tree_iter<T, __node_pointer, difference_type> iterator;
};

template<class T>
struct map
{
  typedef tree<T> __base;
  typedef map_iter<typename __base::iterator> iterator;

  int f();
};

template<class T>
int map<T>::f()
{
  return 0;
}

int main()
{
  return 0;
}
