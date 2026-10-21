// expect exit 0: packed constant data keeps byte offsets before dynamic observers.
#pragma pack(push, 1)
struct Bits { unsigned first : 3; unsigned second : 30; int tail; };
struct Wide { signed char first : 3; signed long long second : 80; signed char tail; };
struct Mixed { char lead; short small; int word; long long wide; float single; double real; long double extended; };
#pragma pack(pop)
extern Bits bits[2];
extern Wide widths[2];
extern Mixed mixed;
int observe() {
  return bits[0].first == 2 && bits[0].second == 1234567 && bits[0].tail == 37 &&
    bits[1].first == 5 && bits[1].second == 7654321 && bits[1].tail == -9 &&
    widths[0].first == -2 && widths[0].second == -3 && widths[0].tail == 37 &&
    widths[1].first == -1 && widths[1].second == -7 && widths[1].tail == -9 &&
    mixed.lead == 11 && mixed.small == -123 && mixed.word == 123456789 &&
    mixed.wide == -1234567890123LL && mixed.single == 1.5f && mixed.real == -2.25 && mixed.extended == 3.5L;
}
int snapshot = observe();
Bits bits[2] = {{2, 1234567, 37}, {5, 7654321, -9}};
Wide widths[2] = {{-2, -3, 37}, {-1, -7, -9}};
Mixed mixed = {11, -123, 123456789, -1234567890123LL, 1.5f, -2.25, 3.5L};
const Mixed& temporary = Mixed{7, -12, 23456, -34567890123LL, 2.5f, -4.25, 6.5L};
int main() {
  return snapshot != 1 || !observe() || temporary.lead != 7 || temporary.small != -12 ||
    temporary.word != 23456 || temporary.wide != -34567890123LL ||
    temporary.single != 2.5f || temporary.real != -4.25 || temporary.extended != 6.5L;
}
