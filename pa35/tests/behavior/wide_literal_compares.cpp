// An extension compared with a literal its source type holds compares at
// that type (host O1+): for an i128 comparison the literal's high half must
// extend its low half, or the literal is out of the source's range.
#include <cstdio>

typedef unsigned __int128 u128;
typedef __int128 s128;

volatile unsigned vu = 5;
volatile int vi = -5;

__attribute__((noinline)) int fits64(unsigned x) { return static_cast<u128>(x) < (static_cast<u128>(1) << 64); }
__attribute__((noinline)) int equal_big(unsigned x) { return static_cast<u128>(x) == ((static_cast<u128>(1) << 64) | 5); }
__attribute__((noinline)) int equal_low(int x) { return static_cast<s128>(x) == static_cast<s128>(0xFFFFFFFFFFFFFFFBull); }
__attribute__((noinline)) int above_negative(int x) { return static_cast<s128>(x) >= -(static_cast<s128>(1) << 64); }
__attribute__((noinline)) int small(unsigned x) { return static_cast<u128>(x) == 5; }   // narrows
__attribute__((noinline)) int negative(int x) { return static_cast<s128>(x) == -5; }    // narrows

int main()
{
	std::printf("%d %d %d %d %d %d\n", fits64(vu), equal_big(vu), equal_low(vi), above_negative(vi), small(vu), negative(vi));
	return 0;
}
