template<class T>
struct optional_ref;

template<class T>
struct optional_ref<T &> {
  T *ptr_;

  T &get() const
  {
    return ptr_ ? *ptr_ : ([]{ int marker = 0; }(), *ptr_);
  }
};

int main()
{
  int value = 7;
  optional_ref<int &> ref = { &value };
  return ref.get() == value ? 0 : 1;
}
