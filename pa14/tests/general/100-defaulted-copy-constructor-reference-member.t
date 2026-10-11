template <class T>
struct RefPair {
  T* first;
  T& second;

  RefPair(T* first_value, T& second_value)
      : first(first_value), second(second_value) {}
  RefPair(const RefPair&) = default;
};

int main()
{
  int value = 7;
  RefPair<int> source(&value, value);
  RefPair<int> copy(source);
  return copy.second - 7;
}
