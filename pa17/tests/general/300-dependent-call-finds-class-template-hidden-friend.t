// VALIDATION: compile-pass
// A dependent call in a function-template specialization finds the hidden
// non-template friend introduced by a class-template specialization.

template<class T>
struct node
{
  friend bool equal(const node&, const node&)
  {
    return true;
  }
};

template<class T>
bool compare(const node<T>& left, const node<T>& right)
{
  return equal(left, right);
}

int main()
{
  node<int> left;
  node<int> right;
  return compare(left, right) ? 0 : 1;
}
