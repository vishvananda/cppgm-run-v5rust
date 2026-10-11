int construction_error = 0;
struct V { virtual void probe() {} };
struct B : virtual V { B(); };
B::B() {
  if(dynamic_cast<void*>(static_cast<V*>(this)) != this)
    construction_error = 1;
}
struct Payload { virtual int read() { return 0; } };
struct Holder { virtual void anchor() {} Payload payload; };
struct D : Holder, B {};
int main() { D d; return construction_error || d.payload.read(); }
