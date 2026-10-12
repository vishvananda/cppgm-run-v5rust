// Frozen types-mode special-member declaration responsibility regression.
struct C{~C();};C::~C()noexcept(false){}
