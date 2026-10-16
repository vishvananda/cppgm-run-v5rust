// N3485 14.8.2p8, 14.8.2.5p5: invalid bounds and abstract array
// elements fail substitution, with no function body demand; expect exit 0
struct Abstract { virtual void member() = 0; };
template<class T> char zero(int (*)[sizeof(T) - 4]);
template<class T> long zero(...);
template<class T> char negative(int (*)[1 - (int)sizeof(T)]);
template<class T> long negative(...);
template<class T> char divide(int (*)[1 / (sizeof(T) - 4)]);
template<class T> long divide(...);
template<class T> char abstract(T (*)[sizeof(T)]);
template<class T> long abstract(...);

int main()
{
  if (sizeof(zero<int>(0)) != sizeof(long)) return 1;
  if (sizeof(negative<int>(0)) != sizeof(long)) return 2;
  if (sizeof(divide<int>(0)) != sizeof(long)) return 3;
  return sizeof(abstract<Abstract>(0)) != sizeof(long);
}
