use bitfield::bitfield;

bitfield! {
    /// CPU internal Bus and Cache configuration register accessed at location
    /// fffe_0130.
    ///
    /// Described in the L64360 Architecture Technical Manual
    ///
    /// Behaviour of most fields is unknown (to me at least)
    pub struct BIUCacheControl(u32);
    impl Debug;

    /// ?? Set to 0 by the BIOS
    pub lock, _: 0;
    /// ?? Set to 0 by the BIOS
    pub inv, _: 1;

    /// ?? The BIOS sets this to 1, then to 0 immediately on boot.
    /// Looking at the PSYQ code, this seems important when clearing the ICache.
    pub tag, _: 2;

    /// Sets the Data cache for scratchpad mode.
    /// On the PS: even if not set, there's no D-Cache.
    pub ram, _: 3;

    /// How many words are loaded per D-Cache miss (2, 4, 8 or 16)
    /// On the PS: useless (set to 0 => 2 words)
    pub dblksz, _: 5, 4;

    /// Enables the Data cache.
    /// On the PS: needed alongside bit 3 to use the scratchpad.
    pub ds, _: 7;

    /// How many words are loaded per I-Cache miss (2, 4, 8 or 16)
    /// On the PS: usually set to 01 (4 words).
    ///   Values 10 and 11 (8 and 16 words) don't work correctly on the PS and,
    ///   according to NoCash, they cause a "Crash" (reboot?).
    ///   Value 00 (2 words) is _most likely_ never used.
    ///   We'll assume 01.
    pub iblksz, _: 9, 8;

    /// "Enable I-Cache Set 0" (seems unused and fixed to 0).
    pub is0, _: 10;

    /// Enables the I-Cache
    /// The BIOS always writes 1 here, it's unlikely anything ever disables it.
    pub is1, _: 11;

    /// ?? Set to 0 by the BIOS
    pub intp, _: 12;
    /// ?? Set to 1 by the BIOS
    pub rdpri, _: 13;
    /// ?? Set to 1 by the BIOS
    pub nopad, _: 14;
    /// ?? Set to 1 by the BIOS
    pub bgnt, _: 15;
    /// ?? Set to 1 by the BIOS
    pub ldsch, _: 16;
    /// ?? Set to 0 by the BIOS
    pub nostr, _: 17;
}
