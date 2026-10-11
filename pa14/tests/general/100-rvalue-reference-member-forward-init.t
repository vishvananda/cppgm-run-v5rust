template <class T>
T&& fwd(T& value) {
  return static_cast<T&&>(value);
}

struct Item {
  int value;
};

struct Holder {
  Item&& ref;

  Holder(Item&& item) : ref(fwd<Item>(item)) {}
};

int main() {
  Item item;
  item.value = 9;
  Holder holder(static_cast<Item&&>(item));
  return holder.ref.value - 9;
}
