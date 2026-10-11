typedef int arr_t[2];

arr_t storage;

arr_t && get()
{
  return static_cast<arr_t &&>(storage);
}

int main()
{
  storage[0] = 7;
  return get()[0] - 7;
}
