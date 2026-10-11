template <class T>
struct RefPair {
  T* first;
  T& second;

  RefPair(T* first_value, T& second_value)
      : first(first_value), second(second_value) {}
  RefPair(RefPair&&) = default;
};

int main()
{
  int value = 7;
  RefPair<int> source(&value, value);
  RefPair<int> moved(static_cast<RefPair<int>&&>(source));
  return moved.second - 7;
}
