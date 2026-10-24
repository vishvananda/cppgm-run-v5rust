typedef _Complex int C;
typedef _Complex short S;
constexpr C& select(C& value, int& calls) { ++calls; return value; }
constexpr S& select_short(S& value, int& calls) { ++calls; return value; }
constexpr bool compound() {
 C value={2,3}; int calls=0;
 select(value,calls)+=C{4,5};
 select(value,calls)-=C{1,2};
 select(value,calls)*=2;
 select(value,calls)/=2;
 return calls==4 && __real__ value==5 && __imag__ value==6;
}
constexpr bool postfix() {
 C value={2,3}; int calls=0;
 C old=select(value,calls)++;
 if (calls!=1 || __real__ old!=2 || __real__ value!=3) return false;
 old=select(value,calls)--;
 if (calls!=2 || __real__ old!=3 || __real__ value!=2) return false;
 ++select(value,calls);
 --select(value,calls);
 return calls==4 && __real__ value==2 && __imag__ value==3;
}
constexpr bool promoted() {
 S value={2,3}; int calls=0;
 select_short(value,calls)+=C{4,5};
 return calls==1 && __real__ value==6 && __imag__ value==8;
}
constexpr C change(C& value) { value=C{20,30}; return C{1,2}; }
constexpr bool aliased_old() {
 C value={2,3}; int calls=0;
 select(value,calls)+=change(value);
 return calls==1 && __real__ value==21 && __imag__ value==32;
}
constexpr bool braced_assignment() {
 C value={2,3}; value={4,5};
 if (__real__ value!=4 || __imag__ value!=0) return false;
 value={};
 return __real__ value==0 && __imag__ value==0;
}
constexpr bool binary_alias() {
 C value={2,3}; int calls=0; C result=select(value,calls)+change(value);
 return calls==1 && __real__ result==21 && __imag__ result==32;
}
constexpr C& change_reference(C& value) { value=C{20,30}; return value; }
constexpr bool equality_alias() {
 C value={2,3}; int calls=0; bool same=select(value,calls)==change_reference(value);
 return same && calls==1;
}
constexpr C change_short(S& value) { value=S{20,30}; return C{1,2}; }
constexpr bool converted_binary() {
 S value={2,3}; C result=value+change_short(value);
 return __real__ result==3 && __imag__ result==5;
}
static_assert(binary_alias(),"same-type binary operand keeps storage");
static_assert(equality_alias(),"equality consumes retained storage");
static_assert(converted_binary(),"changed component type materializes before right operand");
static_assert(braced_assignment(),"frozen first braced operand and empty list");
constexpr bool converted_imaginary() {
 _Complex long value={2,9007199254740993L};
 value+=0.0;
 return __real__ value==2 && __imag__ value==9007199254740992L;
}
static_assert(converted_imaginary(),"saved imaginary value converts to operation precision");
static_assert(compound(),"compound designation once");
static_assert(postfix(),"step designation once and old value");
static_assert(promoted(),"original storage read before promotion");
static_assert(aliased_old(),"frozen old complex storage aliases right operand mutation");
int main() {return 0;}
