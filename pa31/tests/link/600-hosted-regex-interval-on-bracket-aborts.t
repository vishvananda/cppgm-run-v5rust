// SYMPTOM OF a5-18 (auto copy-init skips the copy ctor): libstdc++
// _StateSeq::_M_clone does `auto __dup = _M_nfa[__u];`, bitwise-copying a
// _State that holds an in-place std::function, so a [0-9]{3} pattern
// double-frees; N3485 7.1.6.4/8.5p16
