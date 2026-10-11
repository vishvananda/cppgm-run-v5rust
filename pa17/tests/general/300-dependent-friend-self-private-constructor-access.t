template<class C>
class Ref {
  typedef typename C::storage_type storage_type;
  friend typename C::self;
private:
  Ref(storage_type*, storage_type) {}
};

class Box {
public:
  typedef int storage_type;
  typedef Box self;

  Ref<Box> make() {
    storage_type x = 0;
    return Ref<Box>(&x, 1);
  }
};

int main() {
  Box b;
  b.make();
  return 0;
}
