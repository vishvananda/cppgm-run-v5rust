// N3485 13.3.3.1.5: ranking a list argument completes the class
// parameter's specialization, without instantiating unrelated members.
// expect exit 0
template<class T> struct Message {
  int n;
  template<class U> Message(int v, U&& u) : n(v + u) {}
  void unused() { T::missing(); }
};
struct Payload {
  int n;
  Payload(Message<int>&& message) : n(message.n) {}
};
template<class T> struct Aggregate { T n; };
int read(const Aggregate<int>& a) { return a.n; }
int main() {
  int n = 4;
  Payload p({1, n});
  return p.n == 5 && read({6}) == 6 && read({7}) == 7 ? 0 : 1;
}
