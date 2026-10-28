struct X { X(){} X(const X&){} X(X&&)=delete; };
struct P { X x; };
X by_value(){auto [x]=P{};return x;}
X by_reference(P& p){auto& [x]=p;return x;}
void by_throw(){auto [x]=P{};throw x;}
