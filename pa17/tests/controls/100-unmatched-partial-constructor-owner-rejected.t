// expect reject
// Special-member ownership is checked before the ordinary declarator suffixes.
template<class T>
struct Owner
{
  Owner();
};

template<class T>
Owner<T *>::Owner()
{
}
