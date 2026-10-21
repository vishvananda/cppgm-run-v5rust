// expect exit 0
// 6.7p4 / 12.2p3: constructor failure retries; temporary-dtor failure after
// initialization does not. The completed static is destroyed exactly once.
extern "C" void abort();
int constructed, destroyed, temporaries, constructor_failure = 1, destructor_failure;
struct Temporary {
  Temporary() { ++temporaries; }
  ~Temporary() noexcept(false) {
    --temporaries;
    if (destructor_failure) { destructor_failure = 0; throw 7; }
  }
};
struct Value {
  Value(const Temporary&) {
    if (constructor_failure) { constructor_failure = 0; throw 5; }
    ++constructed;
  }
  ~Value() { ++destroyed; }
};
struct VerifyExit {
  ~VerifyExit() { if (constructed != 1 || destroyed != 1 || temporaries) abort(); }
} verify_exit;
void enter() { static Value value((Temporary())); }
int scalar_calls, reference_calls, target = 23;
int produce(const Temporary&) { ++scalar_calls; return 42; }
int& refer(const Temporary&) { ++reference_calls; return target; }
int scalar() { static int value = produce(Temporary()); return value; }
int& reference() { static int& value = refer(Temporary()); return value; }
int main() {
  int caught = 0;
  try { enter(); } catch (int n) { if (n != 5) return 1; caught = 1; }
  if (!caught || constructed || destroyed || temporaries) return 2;
  destructor_failure = 1;
  caught = 0;
  try { enter(); } catch (int n) { if (n != 7) return 3; caught = 1; }
  if (!caught || constructed != 1 || destroyed || temporaries) return 4;
  enter();
  enter();
  destructor_failure = 1;
  caught = 0;
  try { scalar(); } catch (int n) { if (n != 7) return 5; caught = 1; }
  if (!caught || scalar() != 42 || scalar_calls != 1 || temporaries) return 6;
  destructor_failure = 1;
  caught = 0;
  try { reference(); } catch (int n) { if (n != 7) return 7; caught = 1; }
  if (!caught || &reference() != &target || reference_calls != 1 || temporaries) return 8;
  return constructed != 1 || destroyed || temporaries;
}
