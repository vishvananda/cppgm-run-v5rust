struct Base { int tag; };
struct Derived : Base {};

struct Holder {
  const Base* ptr;
};

struct Owner {
  Derived object;
  Derived* storage;

  Owner() : object(), storage(&object) {
    object.tag = 3;
  }

  Derived*& get() {
    return storage;
  }

  void assign(Holder& holder) {
    holder.ptr = get();
  }
};

int main() {
  Holder holder;
  Owner owner;
  owner.assign(holder);
  return holder.ptr->tag - 3;
}
