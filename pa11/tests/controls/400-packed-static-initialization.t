// expect exit 0: constant initialization precedes an earlier dynamic observer.
#pragma pack(push, 1)
struct Packed { short low : 3; int wide : 30; };
struct Spans { char low : 3; unsigned long long wide : 64; char after; };
#pragma pack(pop)
extern Packed packed;
extern Spans spans;
int observe() {
  return packed.low == 2 && packed.wide == 123456789 && spans.low == 2 &&
    spans.wide == 0x0123456789abcdefULL && spans.after == 5;
}
int snapshot = observe();
Packed packed = {2, 123456789};
Spans spans = {2, 0x0123456789abcdefULL, 5};
int main() { return snapshot != 1 || !observe(); }
