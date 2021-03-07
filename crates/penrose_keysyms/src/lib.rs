//! Auto generated Keysym enum for use with xcb keycodes
use strum::*;

/// X keysym mappings: auto generated from X11/keysymdef.h
#[allow(non_camel_case_types)]
#[derive(AsRefStr, EnumString, EnumIter, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum XKeySym {
    /// XK_BackSpace
    #[strum(serialize = "BackSpace")]
    XK_BackSpace,
    /// XK_Tab
    #[strum(serialize = "Tab")]
    XK_Tab,
    /// XK_Linefeed
    #[strum(serialize = "Linefeed")]
    XK_Linefeed,
    /// XK_Clear
    #[strum(serialize = "Clear")]
    XK_Clear,
    /// XK_Return
    #[strum(serialize = "Return")]
    XK_Return,
    /// XK_Pause
    #[strum(serialize = "Pause")]
    XK_Pause,
    /// XK_Scroll_Lock
    #[strum(serialize = "Scroll_Lock")]
    XK_Scroll_Lock,
    /// XK_Sys_Req
    #[strum(serialize = "Sys_Req")]
    XK_Sys_Req,
    /// XK_Escape
    #[strum(serialize = "Escape")]
    XK_Escape,
    /// XK_Delete
    #[strum(serialize = "Delete")]
    XK_Delete,
    /// XK_Home
    #[strum(serialize = "Home")]
    XK_Home,
    /// XK_Left
    #[strum(serialize = "Left")]
    XK_Left,
    /// XK_Up
    #[strum(serialize = "Up")]
    XK_Up,
    /// XK_Right
    #[strum(serialize = "Right")]
    XK_Right,
    /// XK_Down
    #[strum(serialize = "Down")]
    XK_Down,
    /// XK_Prior
    #[strum(serialize = "Prior")]
    XK_Prior,
    /// XK_Page_Up
    #[strum(serialize = "Page_Up")]
    XK_Page_Up,
    /// XK_Next
    #[strum(serialize = "Next")]
    XK_Next,
    /// XK_Page_Down
    #[strum(serialize = "Page_Down")]
    XK_Page_Down,
    /// XK_End
    #[strum(serialize = "End")]
    XK_End,
    /// XK_Begin
    #[strum(serialize = "Begin")]
    XK_Begin,
    /// XK_Select
    #[strum(serialize = "Select")]
    XK_Select,
    /// XK_Print
    #[strum(serialize = "Print")]
    XK_Print,
    /// XK_Execute
    #[strum(serialize = "Execute")]
    XK_Execute,
    /// XK_Insert
    #[strum(serialize = "Insert")]
    XK_Insert,
    /// XK_Undo
    #[strum(serialize = "Undo")]
    XK_Undo,
    /// XK_Redo
    #[strum(serialize = "Redo")]
    XK_Redo,
    /// XK_Menu
    #[strum(serialize = "Menu")]
    XK_Menu,
    /// XK_Find
    #[strum(serialize = "Find")]
    XK_Find,
    /// XK_Cancel
    #[strum(serialize = "Cancel")]
    XK_Cancel,
    /// XK_Help
    #[strum(serialize = "Help")]
    XK_Help,
    /// XK_Break
    #[strum(serialize = "Break")]
    XK_Break,
    /// XK_Mode_switch
    #[strum(serialize = "Mode_switch")]
    XK_Mode_switch,
    /// XK_script_switch
    #[strum(serialize = "script_switch")]
    XK_script_switch,
    /// XK_Num_Lock
    #[strum(serialize = "Num_Lock")]
    XK_Num_Lock,
    /// XK_KP_Space
    #[strum(serialize = "KP_Space")]
    XK_KP_Space,
    /// XK_KP_Tab
    #[strum(serialize = "KP_Tab")]
    XK_KP_Tab,
    /// XK_KP_Enter
    #[strum(serialize = "KP_Enter")]
    XK_KP_Enter,
    /// XK_KP_F1
    #[strum(serialize = "KP_F1")]
    XK_KP_F1,
    /// XK_KP_F2
    #[strum(serialize = "KP_F2")]
    XK_KP_F2,
    /// XK_KP_F3
    #[strum(serialize = "KP_F3")]
    XK_KP_F3,
    /// XK_KP_F4
    #[strum(serialize = "KP_F4")]
    XK_KP_F4,
    /// XK_KP_Home
    #[strum(serialize = "KP_Home")]
    XK_KP_Home,
    /// XK_KP_Left
    #[strum(serialize = "KP_Left")]
    XK_KP_Left,
    /// XK_KP_Up
    #[strum(serialize = "KP_Up")]
    XK_KP_Up,
    /// XK_KP_Right
    #[strum(serialize = "KP_Right")]
    XK_KP_Right,
    /// XK_KP_Down
    #[strum(serialize = "KP_Down")]
    XK_KP_Down,
    /// XK_KP_Prior
    #[strum(serialize = "KP_Prior")]
    XK_KP_Prior,
    /// XK_KP_Page_Up
    #[strum(serialize = "KP_Page_Up")]
    XK_KP_Page_Up,
    /// XK_KP_Next
    #[strum(serialize = "KP_Next")]
    XK_KP_Next,
    /// XK_KP_Page_Down
    #[strum(serialize = "KP_Page_Down")]
    XK_KP_Page_Down,
    /// XK_KP_End
    #[strum(serialize = "KP_End")]
    XK_KP_End,
    /// XK_KP_Begin
    #[strum(serialize = "KP_Begin")]
    XK_KP_Begin,
    /// XK_KP_Insert
    #[strum(serialize = "KP_Insert")]
    XK_KP_Insert,
    /// XK_KP_Delete
    #[strum(serialize = "KP_Delete")]
    XK_KP_Delete,
    /// XK_KP_Equal
    #[strum(serialize = "KP_Equal")]
    XK_KP_Equal,
    /// XK_KP_Multiply
    #[strum(serialize = "KP_Multiply")]
    XK_KP_Multiply,
    /// XK_KP_Add
    #[strum(serialize = "KP_Add")]
    XK_KP_Add,
    /// XK_KP_Separator
    #[strum(serialize = "KP_Separator")]
    XK_KP_Separator,
    /// XK_KP_Subtract
    #[strum(serialize = "KP_Subtract")]
    XK_KP_Subtract,
    /// XK_KP_Decimal
    #[strum(serialize = "KP_Decimal")]
    XK_KP_Decimal,
    /// XK_KP_Divide
    #[strum(serialize = "KP_Divide")]
    XK_KP_Divide,
    /// XK_KP_0
    #[strum(serialize = "KP_0")]
    XK_KP_0,
    /// XK_KP_1
    #[strum(serialize = "KP_1")]
    XK_KP_1,
    /// XK_KP_2
    #[strum(serialize = "KP_2")]
    XK_KP_2,
    /// XK_KP_3
    #[strum(serialize = "KP_3")]
    XK_KP_3,
    /// XK_KP_4
    #[strum(serialize = "KP_4")]
    XK_KP_4,
    /// XK_KP_5
    #[strum(serialize = "KP_5")]
    XK_KP_5,
    /// XK_KP_6
    #[strum(serialize = "KP_6")]
    XK_KP_6,
    /// XK_KP_7
    #[strum(serialize = "KP_7")]
    XK_KP_7,
    /// XK_KP_8
    #[strum(serialize = "KP_8")]
    XK_KP_8,
    /// XK_KP_9
    #[strum(serialize = "KP_9")]
    XK_KP_9,
    /// XK_F1
    #[strum(serialize = "F1")]
    XK_F1,
    /// XK_F2
    #[strum(serialize = "F2")]
    XK_F2,
    /// XK_F3
    #[strum(serialize = "F3")]
    XK_F3,
    /// XK_F4
    #[strum(serialize = "F4")]
    XK_F4,
    /// XK_F5
    #[strum(serialize = "F5")]
    XK_F5,
    /// XK_F6
    #[strum(serialize = "F6")]
    XK_F6,
    /// XK_F7
    #[strum(serialize = "F7")]
    XK_F7,
    /// XK_F8
    #[strum(serialize = "F8")]
    XK_F8,
    /// XK_F9
    #[strum(serialize = "F9")]
    XK_F9,
    /// XK_F10
    #[strum(serialize = "F10")]
    XK_F10,
    /// XK_F11
    #[strum(serialize = "F11")]
    XK_F11,
    /// XK_L1
    #[strum(serialize = "L1")]
    XK_L1,
    /// XK_F12
    #[strum(serialize = "F12")]
    XK_F12,
    /// XK_L2
    #[strum(serialize = "L2")]
    XK_L2,
    /// XK_F13
    #[strum(serialize = "F13")]
    XK_F13,
    /// XK_L3
    #[strum(serialize = "L3")]
    XK_L3,
    /// XK_F14
    #[strum(serialize = "F14")]
    XK_F14,
    /// XK_L4
    #[strum(serialize = "L4")]
    XK_L4,
    /// XK_F15
    #[strum(serialize = "F15")]
    XK_F15,
    /// XK_L5
    #[strum(serialize = "L5")]
    XK_L5,
    /// XK_F16
    #[strum(serialize = "F16")]
    XK_F16,
    /// XK_L6
    #[strum(serialize = "L6")]
    XK_L6,
    /// XK_F17
    #[strum(serialize = "F17")]
    XK_F17,
    /// XK_L7
    #[strum(serialize = "L7")]
    XK_L7,
    /// XK_F18
    #[strum(serialize = "F18")]
    XK_F18,
    /// XK_L8
    #[strum(serialize = "L8")]
    XK_L8,
    /// XK_F19
    #[strum(serialize = "F19")]
    XK_F19,
    /// XK_L9
    #[strum(serialize = "L9")]
    XK_L9,
    /// XK_F20
    #[strum(serialize = "F20")]
    XK_F20,
    /// XK_L10
    #[strum(serialize = "L10")]
    XK_L10,
    /// XK_F21
    #[strum(serialize = "F21")]
    XK_F21,
    /// XK_R1
    #[strum(serialize = "R1")]
    XK_R1,
    /// XK_F22
    #[strum(serialize = "F22")]
    XK_F22,
    /// XK_R2
    #[strum(serialize = "R2")]
    XK_R2,
    /// XK_F23
    #[strum(serialize = "F23")]
    XK_F23,
    /// XK_R3
    #[strum(serialize = "R3")]
    XK_R3,
    /// XK_F24
    #[strum(serialize = "F24")]
    XK_F24,
    /// XK_R4
    #[strum(serialize = "R4")]
    XK_R4,
    /// XK_F25
    #[strum(serialize = "F25")]
    XK_F25,
    /// XK_R5
    #[strum(serialize = "R5")]
    XK_R5,
    /// XK_F26
    #[strum(serialize = "F26")]
    XK_F26,
    /// XK_R6
    #[strum(serialize = "R6")]
    XK_R6,
    /// XK_F27
    #[strum(serialize = "F27")]
    XK_F27,
    /// XK_R7
    #[strum(serialize = "R7")]
    XK_R7,
    /// XK_F28
    #[strum(serialize = "F28")]
    XK_F28,
    /// XK_R8
    #[strum(serialize = "R8")]
    XK_R8,
    /// XK_F29
    #[strum(serialize = "F29")]
    XK_F29,
    /// XK_R9
    #[strum(serialize = "R9")]
    XK_R9,
    /// XK_F30
    #[strum(serialize = "F30")]
    XK_F30,
    /// XK_R10
    #[strum(serialize = "R10")]
    XK_R10,
    /// XK_F31
    #[strum(serialize = "F31")]
    XK_F31,
    /// XK_R11
    #[strum(serialize = "R11")]
    XK_R11,
    /// XK_F32
    #[strum(serialize = "F32")]
    XK_F32,
    /// XK_R12
    #[strum(serialize = "R12")]
    XK_R12,
    /// XK_F33
    #[strum(serialize = "F33")]
    XK_F33,
    /// XK_R13
    #[strum(serialize = "R13")]
    XK_R13,
    /// XK_F34
    #[strum(serialize = "F34")]
    XK_F34,
    /// XK_R14
    #[strum(serialize = "R14")]
    XK_R14,
    /// XK_F35
    #[strum(serialize = "F35")]
    XK_F35,
    /// XK_R15
    #[strum(serialize = "R15")]
    XK_R15,
    /// XK_Shift_L
    #[strum(serialize = "Shift_L")]
    XK_Shift_L,
    /// XK_Shift_R
    #[strum(serialize = "Shift_R")]
    XK_Shift_R,
    /// XK_Control_L
    #[strum(serialize = "Control_L")]
    XK_Control_L,
    /// XK_Control_R
    #[strum(serialize = "Control_R")]
    XK_Control_R,
    /// XK_Caps_Lock
    #[strum(serialize = "Caps_Lock")]
    XK_Caps_Lock,
    /// XK_Shift_Lock
    #[strum(serialize = "Shift_Lock")]
    XK_Shift_Lock,
    /// XK_Meta_L
    #[strum(serialize = "Meta_L")]
    XK_Meta_L,
    /// XK_Meta_R
    #[strum(serialize = "Meta_R")]
    XK_Meta_R,
    /// XK_Alt_L
    #[strum(serialize = "Alt_L")]
    XK_Alt_L,
    /// XK_Alt_R
    #[strum(serialize = "Alt_R")]
    XK_Alt_R,
    /// XK_Super_L
    #[strum(serialize = "Super_L")]
    XK_Super_L,
    /// XK_Super_R
    #[strum(serialize = "Super_R")]
    XK_Super_R,
    /// XK_Hyper_L
    #[strum(serialize = "Hyper_L")]
    XK_Hyper_L,
    /// XK_Hyper_R
    #[strum(serialize = "Hyper_R")]
    XK_Hyper_R,
    /// XK_ISO_Lock
    #[strum(serialize = "ISO_Lock")]
    XK_ISO_Lock,
    /// XK_ISO_Level2_Latch
    #[strum(serialize = "ISO_Level2_Latch")]
    XK_ISO_Level2_Latch,
    /// XK_ISO_Level3_Shift
    #[strum(serialize = "ISO_Level3_Shift")]
    XK_ISO_Level3_Shift,
    /// XK_ISO_Level3_Latch
    #[strum(serialize = "ISO_Level3_Latch")]
    XK_ISO_Level3_Latch,
    /// XK_ISO_Level3_Lock
    #[strum(serialize = "ISO_Level3_Lock")]
    XK_ISO_Level3_Lock,
    /// XK_ISO_Level5_Shift
    #[strum(serialize = "ISO_Level5_Shift")]
    XK_ISO_Level5_Shift,
    /// XK_ISO_Level5_Latch
    #[strum(serialize = "ISO_Level5_Latch")]
    XK_ISO_Level5_Latch,
    /// XK_ISO_Level5_Lock
    #[strum(serialize = "ISO_Level5_Lock")]
    XK_ISO_Level5_Lock,
    /// XK_ISO_Left_Tab
    #[strum(serialize = "ISO_Left_Tab")]
    XK_ISO_Left_Tab,
    /// XK_ISO_Partial_Space_Left
    #[strum(serialize = "ISO_Partial_Space_Left")]
    XK_ISO_Partial_Space_Left,
    /// XK_ISO_Partial_Space_Right
    #[strum(serialize = "ISO_Partial_Space_Right")]
    XK_ISO_Partial_Space_Right,
    /// XK_ISO_Set_Margin_Left
    #[strum(serialize = "ISO_Set_Margin_Left")]
    XK_ISO_Set_Margin_Left,
    /// XK_ISO_Set_Margin_Right
    #[strum(serialize = "ISO_Set_Margin_Right")]
    XK_ISO_Set_Margin_Right,
    /// XK_ISO_Continuous_Underline
    #[strum(serialize = "ISO_Continuous_Underline")]
    XK_ISO_Continuous_Underline,
    /// XK_ISO_Discontinuous_Underline
    #[strum(serialize = "ISO_Discontinuous_Underline")]
    XK_ISO_Discontinuous_Underline,
    /// XK_ISO_Emphasize
    #[strum(serialize = "ISO_Emphasize")]
    XK_ISO_Emphasize,
    /// XK_ISO_Center_Object
    #[strum(serialize = "ISO_Center_Object")]
    XK_ISO_Center_Object,
    /// XK_ISO_Enter
    #[strum(serialize = "ISO_Enter")]
    XK_ISO_Enter,
    /// XK_Terminate_Server
    #[strum(serialize = "Terminate_Server")]
    XK_Terminate_Server,
    /// XK_ch
    #[strum(serialize = "ch")]
    XK_ch,
    /// XK_Ch
    #[strum(serialize = "Ch")]
    XK_Ch,
    /// XK_CH
    #[strum(serialize = "CH")]
    XK_CH,
    /// XK_c_h
    #[strum(serialize = "c_h")]
    XK_c_h,
    /// XK_C_h
    #[strum(serialize = "C_h")]
    XK_C_h,
    /// XK_C_H
    #[strum(serialize = "C_H")]
    XK_C_H,
    /// XK_3270_Duplicate
    #[strum(serialize = "3270_Duplicate")]
    XK_3270_Duplicate,
    /// XK_3270_FieldMark
    #[strum(serialize = "3270_FieldMark")]
    XK_3270_FieldMark,
    /// XK_3270_Right2
    #[strum(serialize = "3270_Right2")]
    XK_3270_Right2,
    /// XK_3270_Left2
    #[strum(serialize = "3270_Left2")]
    XK_3270_Left2,
    /// XK_3270_BackTab
    #[strum(serialize = "3270_BackTab")]
    XK_3270_BackTab,
    /// XK_3270_EraseEOF
    #[strum(serialize = "3270_EraseEOF")]
    XK_3270_EraseEOF,
    /// XK_3270_EraseInput
    #[strum(serialize = "3270_EraseInput")]
    XK_3270_EraseInput,
    /// XK_3270_Reset
    #[strum(serialize = "3270_Reset")]
    XK_3270_Reset,
    /// XK_3270_Quit
    #[strum(serialize = "3270_Quit")]
    XK_3270_Quit,
    /// XK_3270_PA1
    #[strum(serialize = "3270_PA1")]
    XK_3270_PA1,
    /// XK_3270_PA2
    #[strum(serialize = "3270_PA2")]
    XK_3270_PA2,
    /// XK_3270_PA3
    #[strum(serialize = "3270_PA3")]
    XK_3270_PA3,
    /// XK_3270_Test
    #[strum(serialize = "3270_Test")]
    XK_3270_Test,
    /// XK_3270_Attn
    #[strum(serialize = "3270_Attn")]
    XK_3270_Attn,
    /// XK_3270_CursorBlink
    #[strum(serialize = "3270_CursorBlink")]
    XK_3270_CursorBlink,
    /// XK_3270_AltCursor
    #[strum(serialize = "3270_AltCursor")]
    XK_3270_AltCursor,
    /// XK_3270_KeyClick
    #[strum(serialize = "3270_KeyClick")]
    XK_3270_KeyClick,
    /// XK_3270_Jump
    #[strum(serialize = "3270_Jump")]
    XK_3270_Jump,
    /// XK_3270_Ident
    #[strum(serialize = "3270_Ident")]
    XK_3270_Ident,
    /// XK_3270_Rule
    #[strum(serialize = "3270_Rule")]
    XK_3270_Rule,
    /// XK_3270_Copy
    #[strum(serialize = "3270_Copy")]
    XK_3270_Copy,
    /// XK_3270_Play
    #[strum(serialize = "3270_Play")]
    XK_3270_Play,
    /// XK_3270_Setup
    #[strum(serialize = "3270_Setup")]
    XK_3270_Setup,
    /// XK_3270_Record
    #[strum(serialize = "3270_Record")]
    XK_3270_Record,
    /// XK_3270_DeleteWord
    #[strum(serialize = "3270_DeleteWord")]
    XK_3270_DeleteWord,
    /// XK_3270_ExSelect
    #[strum(serialize = "3270_ExSelect")]
    XK_3270_ExSelect,
    /// XK_3270_CursorSelect
    #[strum(serialize = "3270_CursorSelect")]
    XK_3270_CursorSelect,
    /// XK_3270_Enter
    #[strum(serialize = "3270_Enter")]
    XK_3270_Enter,
    /// XK_space
    #[strum(serialize = "space")]
    XK_space,
    /// XK_exclam
    #[strum(serialize = "exclam")]
    XK_exclam,
    /// XK_quotedbl
    #[strum(serialize = "quotedbl")]
    XK_quotedbl,
    /// XK_numbersign
    #[strum(serialize = "numbersign")]
    XK_numbersign,
    /// XK_dollar
    #[strum(serialize = "dollar")]
    XK_dollar,
    /// XK_percent
    #[strum(serialize = "percent")]
    XK_percent,
    /// XK_ampersand
    #[strum(serialize = "ampersand")]
    XK_ampersand,
    /// XK_apostrophe
    #[strum(serialize = "apostrophe")]
    XK_apostrophe,
    /// XK_quoteright
    #[strum(serialize = "quoteright")]
    XK_quoteright,
    /// XK_parenleft
    #[strum(serialize = "parenleft")]
    XK_parenleft,
    /// XK_parenright
    #[strum(serialize = "parenright")]
    XK_parenright,
    /// XK_asterisk
    #[strum(serialize = "asterisk")]
    XK_asterisk,
    /// XK_plus
    #[strum(serialize = "plus")]
    XK_plus,
    /// XK_comma
    #[strum(serialize = "comma")]
    XK_comma,
    /// XK_minus
    #[strum(serialize = "minus")]
    XK_minus,
    /// XK_period
    #[strum(serialize = "period")]
    XK_period,
    /// XK_slash
    #[strum(serialize = "slash")]
    XK_slash,
    /// XK_0
    #[strum(serialize = "0")]
    XK_0,
    /// XK_1
    #[strum(serialize = "1")]
    XK_1,
    /// XK_2
    #[strum(serialize = "2")]
    XK_2,
    /// XK_3
    #[strum(serialize = "3")]
    XK_3,
    /// XK_4
    #[strum(serialize = "4")]
    XK_4,
    /// XK_5
    #[strum(serialize = "5")]
    XK_5,
    /// XK_6
    #[strum(serialize = "6")]
    XK_6,
    /// XK_7
    #[strum(serialize = "7")]
    XK_7,
    /// XK_8
    #[strum(serialize = "8")]
    XK_8,
    /// XK_9
    #[strum(serialize = "9")]
    XK_9,
    /// XK_colon
    #[strum(serialize = "colon")]
    XK_colon,
    /// XK_semicolon
    #[strum(serialize = "semicolon")]
    XK_semicolon,
    /// XK_less
    #[strum(serialize = "less")]
    XK_less,
    /// XK_equal
    #[strum(serialize = "equal")]
    XK_equal,
    /// XK_greater
    #[strum(serialize = "greater")]
    XK_greater,
    /// XK_question
    #[strum(serialize = "question")]
    XK_question,
    /// XK_at
    #[strum(serialize = "at")]
    XK_at,
    /// XK_A
    #[strum(serialize = "A")]
    XK_A,
    /// XK_B
    #[strum(serialize = "B")]
    XK_B,
    /// XK_C
    #[strum(serialize = "C")]
    XK_C,
    /// XK_D
    #[strum(serialize = "D")]
    XK_D,
    /// XK_E
    #[strum(serialize = "E")]
    XK_E,
    /// XK_F
    #[strum(serialize = "F")]
    XK_F,
    /// XK_G
    #[strum(serialize = "G")]
    XK_G,
    /// XK_H
    #[strum(serialize = "H")]
    XK_H,
    /// XK_I
    #[strum(serialize = "I")]
    XK_I,
    /// XK_J
    #[strum(serialize = "J")]
    XK_J,
    /// XK_K
    #[strum(serialize = "K")]
    XK_K,
    /// XK_L
    #[strum(serialize = "L")]
    XK_L,
    /// XK_M
    #[strum(serialize = "M")]
    XK_M,
    /// XK_N
    #[strum(serialize = "N")]
    XK_N,
    /// XK_O
    #[strum(serialize = "O")]
    XK_O,
    /// XK_P
    #[strum(serialize = "P")]
    XK_P,
    /// XK_Q
    #[strum(serialize = "Q")]
    XK_Q,
    /// XK_R
    #[strum(serialize = "R")]
    XK_R,
    /// XK_S
    #[strum(serialize = "S")]
    XK_S,
    /// XK_T
    #[strum(serialize = "T")]
    XK_T,
    /// XK_U
    #[strum(serialize = "U")]
    XK_U,
    /// XK_V
    #[strum(serialize = "V")]
    XK_V,
    /// XK_W
    #[strum(serialize = "W")]
    XK_W,
    /// XK_X
    #[strum(serialize = "X")]
    XK_X,
    /// XK_Y
    #[strum(serialize = "Y")]
    XK_Y,
    /// XK_Z
    #[strum(serialize = "Z")]
    XK_Z,
    /// XK_bracketleft
    #[strum(serialize = "bracketleft")]
    XK_bracketleft,
    /// XK_backslash
    #[strum(serialize = "backslash")]
    XK_backslash,
    /// XK_bracketright
    #[strum(serialize = "bracketright")]
    XK_bracketright,
    /// XK_asciicircum
    #[strum(serialize = "asciicircum")]
    XK_asciicircum,
    /// XK_underscore
    #[strum(serialize = "underscore")]
    XK_underscore,
    /// XK_grave
    #[strum(serialize = "grave")]
    XK_grave,
    /// XK_quoteleft
    #[strum(serialize = "quoteleft")]
    XK_quoteleft,
    /// XK_a
    #[strum(serialize = "a")]
    XK_a,
    /// XK_b
    #[strum(serialize = "b")]
    XK_b,
    /// XK_c
    #[strum(serialize = "c")]
    XK_c,
    /// XK_d
    #[strum(serialize = "d")]
    XK_d,
    /// XK_e
    #[strum(serialize = "e")]
    XK_e,
    /// XK_f
    #[strum(serialize = "f")]
    XK_f,
    /// XK_g
    #[strum(serialize = "g")]
    XK_g,
    /// XK_h
    #[strum(serialize = "h")]
    XK_h,
    /// XK_i
    #[strum(serialize = "i")]
    XK_i,
    /// XK_j
    #[strum(serialize = "j")]
    XK_j,
    /// XK_k
    #[strum(serialize = "k")]
    XK_k,
    /// XK_l
    #[strum(serialize = "l")]
    XK_l,
    /// XK_m
    #[strum(serialize = "m")]
    XK_m,
    /// XK_n
    #[strum(serialize = "n")]
    XK_n,
    /// XK_o
    #[strum(serialize = "o")]
    XK_o,
    /// XK_p
    #[strum(serialize = "p")]
    XK_p,
    /// XK_q
    #[strum(serialize = "q")]
    XK_q,
    /// XK_r
    #[strum(serialize = "r")]
    XK_r,
    /// XK_s
    #[strum(serialize = "s")]
    XK_s,
    /// XK_t
    #[strum(serialize = "t")]
    XK_t,
    /// XK_u
    #[strum(serialize = "u")]
    XK_u,
    /// XK_v
    #[strum(serialize = "v")]
    XK_v,
    /// XK_w
    #[strum(serialize = "w")]
    XK_w,
    /// XK_x
    #[strum(serialize = "x")]
    XK_x,
    /// XK_y
    #[strum(serialize = "y")]
    XK_y,
    /// XK_z
    #[strum(serialize = "z")]
    XK_z,
    /// XK_braceleft
    #[strum(serialize = "braceleft")]
    XK_braceleft,
    /// XK_bar
    #[strum(serialize = "bar")]
    XK_bar,
    /// XK_braceright
    #[strum(serialize = "braceright")]
    XK_braceright,
    /// XK_asciitilde
    #[strum(serialize = "asciitilde")]
    XK_asciitilde,
    /// XK_nobreakspace
    #[strum(serialize = "nobreakspace")]
    XK_nobreakspace,
    /// XK_exclamdown
    #[strum(serialize = "exclamdown")]
    XK_exclamdown,
    /// XK_cent
    #[strum(serialize = "cent")]
    XK_cent,
    /// XK_sterling
    #[strum(serialize = "sterling")]
    XK_sterling,
    /// XK_currency
    #[strum(serialize = "currency")]
    XK_currency,
    /// XK_yen
    #[strum(serialize = "yen")]
    XK_yen,
    /// XK_brokenbar
    #[strum(serialize = "brokenbar")]
    XK_brokenbar,
    /// XK_section
    #[strum(serialize = "section")]
    XK_section,
    /// XK_diaeresis
    #[strum(serialize = "diaeresis")]
    XK_diaeresis,
    /// XK_copyright
    #[strum(serialize = "copyright")]
    XK_copyright,
    /// XK_ordfeminine
    #[strum(serialize = "ordfeminine")]
    XK_ordfeminine,
    /// XK_guillemotleft
    #[strum(serialize = "guillemotleft")]
    XK_guillemotleft,
    /// XK_notsign
    #[strum(serialize = "notsign")]
    XK_notsign,
    /// XK_hyphen
    #[strum(serialize = "hyphen")]
    XK_hyphen,
    /// XK_registered
    #[strum(serialize = "registered")]
    XK_registered,
    /// XK_macron
    #[strum(serialize = "macron")]
    XK_macron,
    /// XK_degree
    #[strum(serialize = "degree")]
    XK_degree,
    /// XK_plusminus
    #[strum(serialize = "plusminus")]
    XK_plusminus,
    /// XK_acute
    #[strum(serialize = "acute")]
    XK_acute,
    /// XK_mu
    #[strum(serialize = "mu")]
    XK_mu,
    /// XK_paragraph
    #[strum(serialize = "paragraph")]
    XK_paragraph,
    /// XK_periodcentered
    #[strum(serialize = "periodcentered")]
    XK_periodcentered,
    /// XK_cedilla
    #[strum(serialize = "cedilla")]
    XK_cedilla,
    /// XK_masculine
    #[strum(serialize = "masculine")]
    XK_masculine,
    /// XK_guillemotright
    #[strum(serialize = "guillemotright")]
    XK_guillemotright,
    /// XK_onequarter
    #[strum(serialize = "onequarter")]
    XK_onequarter,
    /// XK_onehalf
    #[strum(serialize = "onehalf")]
    XK_onehalf,
    /// XK_threequarters
    #[strum(serialize = "threequarters")]
    XK_threequarters,
    /// XK_questiondown
    #[strum(serialize = "questiondown")]
    XK_questiondown,
    /// XK_Aacute
    #[strum(serialize = "Aacute")]
    XK_Aacute,
    /// XK_Atilde
    #[strum(serialize = "Atilde")]
    XK_Atilde,
    /// XK_Adiaeresis
    #[strum(serialize = "Adiaeresis")]
    XK_Adiaeresis,
    /// XK_Aring
    #[strum(serialize = "Aring")]
    XK_Aring,
    /// XK_AE
    #[strum(serialize = "AE")]
    XK_AE,
    /// XK_Ccedilla
    #[strum(serialize = "Ccedilla")]
    XK_Ccedilla,
    /// XK_Eacute
    #[strum(serialize = "Eacute")]
    XK_Eacute,
    /// XK_Ediaeresis
    #[strum(serialize = "Ediaeresis")]
    XK_Ediaeresis,
    /// XK_Iacute
    #[strum(serialize = "Iacute")]
    XK_Iacute,
    /// XK_Idiaeresis
    #[strum(serialize = "Idiaeresis")]
    XK_Idiaeresis,
    /// XK_ETH
    #[strum(serialize = "ETH")]
    XK_ETH,
    /// XK_Eth
    #[strum(serialize = "Eth")]
    XK_Eth,
    /// XK_Ntilde
    #[strum(serialize = "Ntilde")]
    XK_Ntilde,
    /// XK_Oacute
    #[strum(serialize = "Oacute")]
    XK_Oacute,
    /// XK_Otilde
    #[strum(serialize = "Otilde")]
    XK_Otilde,
    /// XK_Odiaeresis
    #[strum(serialize = "Odiaeresis")]
    XK_Odiaeresis,
    /// XK_multiply
    #[strum(serialize = "multiply")]
    XK_multiply,
    /// XK_Oslash
    #[strum(serialize = "Oslash")]
    XK_Oslash,
    /// XK_Ooblique
    #[strum(serialize = "Ooblique")]
    XK_Ooblique,
    /// XK_Uacute
    #[strum(serialize = "Uacute")]
    XK_Uacute,
    /// XK_Udiaeresis
    #[strum(serialize = "Udiaeresis")]
    XK_Udiaeresis,
    /// XK_Yacute
    #[strum(serialize = "Yacute")]
    XK_Yacute,
    /// XK_ssharp
    #[strum(serialize = "ssharp")]
    XK_ssharp,
    /// XK_aacute
    #[strum(serialize = "aacute")]
    XK_aacute,
    /// XK_atilde
    #[strum(serialize = "atilde")]
    XK_atilde,
    /// XK_adiaeresis
    #[strum(serialize = "adiaeresis")]
    XK_adiaeresis,
    /// XK_aring
    #[strum(serialize = "aring")]
    XK_aring,
    /// XK_ae
    #[strum(serialize = "ae")]
    XK_ae,
    /// XK_ccedilla
    #[strum(serialize = "ccedilla")]
    XK_ccedilla,
    /// XK_eacute
    #[strum(serialize = "eacute")]
    XK_eacute,
    /// XK_ediaeresis
    #[strum(serialize = "ediaeresis")]
    XK_ediaeresis,
    /// XK_iacute
    #[strum(serialize = "iacute")]
    XK_iacute,
    /// XK_idiaeresis
    #[strum(serialize = "idiaeresis")]
    XK_idiaeresis,
    /// XK_eth
    #[strum(serialize = "eth")]
    XK_eth,
    /// XK_ntilde
    #[strum(serialize = "ntilde")]
    XK_ntilde,
    /// XK_oacute
    #[strum(serialize = "oacute")]
    XK_oacute,
    /// XK_otilde
    #[strum(serialize = "otilde")]
    XK_otilde,
    /// XK_odiaeresis
    #[strum(serialize = "odiaeresis")]
    XK_odiaeresis,
    /// XK_division
    #[strum(serialize = "division")]
    XK_division,
    /// XK_oslash
    #[strum(serialize = "oslash")]
    XK_oslash,
    /// XK_ooblique
    #[strum(serialize = "ooblique")]
    XK_ooblique,
    /// XK_uacute
    #[strum(serialize = "uacute")]
    XK_uacute,
    /// XK_udiaeresis
    #[strum(serialize = "udiaeresis")]
    XK_udiaeresis,
    /// XK_yacute
    #[strum(serialize = "yacute")]
    XK_yacute,
    /// XK_ydiaeresis
    #[strum(serialize = "ydiaeresis")]
    XK_ydiaeresis,
    /// XK_Aogonek
    #[strum(serialize = "Aogonek")]
    XK_Aogonek,
    /// XK_breve
    #[strum(serialize = "breve")]
    XK_breve,
    /// XK_Lstroke
    #[strum(serialize = "Lstroke")]
    XK_Lstroke,
    /// XK_Lcaron
    #[strum(serialize = "Lcaron")]
    XK_Lcaron,
    /// XK_Sacute
    #[strum(serialize = "Sacute")]
    XK_Sacute,
    /// XK_Scaron
    #[strum(serialize = "Scaron")]
    XK_Scaron,
    /// XK_Scedilla
    #[strum(serialize = "Scedilla")]
    XK_Scedilla,
    /// XK_Tcaron
    #[strum(serialize = "Tcaron")]
    XK_Tcaron,
    /// XK_Zacute
    #[strum(serialize = "Zacute")]
    XK_Zacute,
    /// XK_Zcaron
    #[strum(serialize = "Zcaron")]
    XK_Zcaron,
    /// XK_aogonek
    #[strum(serialize = "aogonek")]
    XK_aogonek,
    /// XK_ogonek
    #[strum(serialize = "ogonek")]
    XK_ogonek,
    /// XK_lstroke
    #[strum(serialize = "lstroke")]
    XK_lstroke,
    /// XK_lcaron
    #[strum(serialize = "lcaron")]
    XK_lcaron,
    /// XK_sacute
    #[strum(serialize = "sacute")]
    XK_sacute,
    /// XK_caron
    #[strum(serialize = "caron")]
    XK_caron,
    /// XK_scaron
    #[strum(serialize = "scaron")]
    XK_scaron,
    /// XK_scedilla
    #[strum(serialize = "scedilla")]
    XK_scedilla,
    /// XK_tcaron
    #[strum(serialize = "tcaron")]
    XK_tcaron,
    /// XK_zacute
    #[strum(serialize = "zacute")]
    XK_zacute,
    /// XK_doubleacute
    #[strum(serialize = "doubleacute")]
    XK_doubleacute,
    /// XK_zcaron
    #[strum(serialize = "zcaron")]
    XK_zcaron,
    /// XK_Racute
    #[strum(serialize = "Racute")]
    XK_Racute,
    /// XK_Abreve
    #[strum(serialize = "Abreve")]
    XK_Abreve,
    /// XK_Lacute
    #[strum(serialize = "Lacute")]
    XK_Lacute,
    /// XK_Cacute
    #[strum(serialize = "Cacute")]
    XK_Cacute,
    /// XK_Ccaron
    #[strum(serialize = "Ccaron")]
    XK_Ccaron,
    /// XK_Eogonek
    #[strum(serialize = "Eogonek")]
    XK_Eogonek,
    /// XK_Ecaron
    #[strum(serialize = "Ecaron")]
    XK_Ecaron,
    /// XK_Dcaron
    #[strum(serialize = "Dcaron")]
    XK_Dcaron,
    /// XK_Dstroke
    #[strum(serialize = "Dstroke")]
    XK_Dstroke,
    /// XK_Nacute
    #[strum(serialize = "Nacute")]
    XK_Nacute,
    /// XK_Ncaron
    #[strum(serialize = "Ncaron")]
    XK_Ncaron,
    /// XK_Odoubleacute
    #[strum(serialize = "Odoubleacute")]
    XK_Odoubleacute,
    /// XK_Rcaron
    #[strum(serialize = "Rcaron")]
    XK_Rcaron,
    /// XK_Uring
    #[strum(serialize = "Uring")]
    XK_Uring,
    /// XK_Udoubleacute
    #[strum(serialize = "Udoubleacute")]
    XK_Udoubleacute,
    /// XK_Tcedilla
    #[strum(serialize = "Tcedilla")]
    XK_Tcedilla,
    /// XK_racute
    #[strum(serialize = "racute")]
    XK_racute,
    /// XK_abreve
    #[strum(serialize = "abreve")]
    XK_abreve,
    /// XK_lacute
    #[strum(serialize = "lacute")]
    XK_lacute,
    /// XK_cacute
    #[strum(serialize = "cacute")]
    XK_cacute,
    /// XK_ccaron
    #[strum(serialize = "ccaron")]
    XK_ccaron,
    /// XK_eogonek
    #[strum(serialize = "eogonek")]
    XK_eogonek,
    /// XK_ecaron
    #[strum(serialize = "ecaron")]
    XK_ecaron,
    /// XK_dcaron
    #[strum(serialize = "dcaron")]
    XK_dcaron,
    /// XK_dstroke
    #[strum(serialize = "dstroke")]
    XK_dstroke,
    /// XK_nacute
    #[strum(serialize = "nacute")]
    XK_nacute,
    /// XK_ncaron
    #[strum(serialize = "ncaron")]
    XK_ncaron,
    /// XK_odoubleacute
    #[strum(serialize = "odoubleacute")]
    XK_odoubleacute,
    /// XK_rcaron
    #[strum(serialize = "rcaron")]
    XK_rcaron,
    /// XK_uring
    #[strum(serialize = "uring")]
    XK_uring,
    /// XK_udoubleacute
    #[strum(serialize = "udoubleacute")]
    XK_udoubleacute,
    /// XK_tcedilla
    #[strum(serialize = "tcedilla")]
    XK_tcedilla,
    /// XK_Hstroke
    #[strum(serialize = "Hstroke")]
    XK_Hstroke,
    /// XK_Gbreve
    #[strum(serialize = "Gbreve")]
    XK_Gbreve,
    /// XK_hstroke
    #[strum(serialize = "hstroke")]
    XK_hstroke,
    /// XK_idotless
    #[strum(serialize = "idotless")]
    XK_idotless,
    /// XK_gbreve
    #[strum(serialize = "gbreve")]
    XK_gbreve,
    /// XK_Ubreve
    #[strum(serialize = "Ubreve")]
    XK_Ubreve,
    /// XK_ubreve
    #[strum(serialize = "ubreve")]
    XK_ubreve,
    /// XK_kra
    #[strum(serialize = "kra")]
    XK_kra,
    /// XK_kappa
    #[strum(serialize = "kappa")]
    XK_kappa,
    /// XK_Rcedilla
    #[strum(serialize = "Rcedilla")]
    XK_Rcedilla,
    /// XK_Itilde
    #[strum(serialize = "Itilde")]
    XK_Itilde,
    /// XK_Lcedilla
    #[strum(serialize = "Lcedilla")]
    XK_Lcedilla,
    /// XK_Emacron
    #[strum(serialize = "Emacron")]
    XK_Emacron,
    /// XK_Gcedilla
    #[strum(serialize = "Gcedilla")]
    XK_Gcedilla,
    /// XK_Tslash
    #[strum(serialize = "Tslash")]
    XK_Tslash,
    /// XK_rcedilla
    #[strum(serialize = "rcedilla")]
    XK_rcedilla,
    /// XK_itilde
    #[strum(serialize = "itilde")]
    XK_itilde,
    /// XK_lcedilla
    #[strum(serialize = "lcedilla")]
    XK_lcedilla,
    /// XK_emacron
    #[strum(serialize = "emacron")]
    XK_emacron,
    /// XK_gcedilla
    #[strum(serialize = "gcedilla")]
    XK_gcedilla,
    /// XK_tslash
    #[strum(serialize = "tslash")]
    XK_tslash,
    /// XK_ENG
    #[strum(serialize = "ENG")]
    XK_ENG,
    /// XK_eng
    #[strum(serialize = "eng")]
    XK_eng,
    /// XK_Amacron
    #[strum(serialize = "Amacron")]
    XK_Amacron,
    /// XK_Iogonek
    #[strum(serialize = "Iogonek")]
    XK_Iogonek,
    /// XK_Imacron
    #[strum(serialize = "Imacron")]
    XK_Imacron,
    /// XK_Ncedilla
    #[strum(serialize = "Ncedilla")]
    XK_Ncedilla,
    /// XK_Omacron
    #[strum(serialize = "Omacron")]
    XK_Omacron,
    /// XK_Kcedilla
    #[strum(serialize = "Kcedilla")]
    XK_Kcedilla,
    /// XK_Uogonek
    #[strum(serialize = "Uogonek")]
    XK_Uogonek,
    /// XK_Utilde
    #[strum(serialize = "Utilde")]
    XK_Utilde,
    /// XK_Umacron
    #[strum(serialize = "Umacron")]
    XK_Umacron,
    /// XK_amacron
    #[strum(serialize = "amacron")]
    XK_amacron,
    /// XK_iogonek
    #[strum(serialize = "iogonek")]
    XK_iogonek,
    /// XK_imacron
    #[strum(serialize = "imacron")]
    XK_imacron,
    /// XK_ncedilla
    #[strum(serialize = "ncedilla")]
    XK_ncedilla,
    /// XK_omacron
    #[strum(serialize = "omacron")]
    XK_omacron,
    /// XK_kcedilla
    #[strum(serialize = "kcedilla")]
    XK_kcedilla,
    /// XK_uogonek
    #[strum(serialize = "uogonek")]
    XK_uogonek,
    /// XK_utilde
    #[strum(serialize = "utilde")]
    XK_utilde,
    /// XK_umacron
    #[strum(serialize = "umacron")]
    XK_umacron,
    /// XK_Wacute
    #[strum(serialize = "Wacute")]
    XK_Wacute,
    /// XK_wacute
    #[strum(serialize = "wacute")]
    XK_wacute,
    /// XK_Wdiaeresis
    #[strum(serialize = "Wdiaeresis")]
    XK_Wdiaeresis,
    /// XK_wdiaeresis
    #[strum(serialize = "wdiaeresis")]
    XK_wdiaeresis,
    /// XK_OE
    #[strum(serialize = "OE")]
    XK_OE,
    /// XK_oe
    #[strum(serialize = "oe")]
    XK_oe,
    /// XK_Ydiaeresis
    #[strum(serialize = "Ydiaeresis")]
    XK_Ydiaeresis,
    /// XK_overline
    #[strum(serialize = "overline")]
    XK_overline,
    /// XK_prolongedsound
    #[strum(serialize = "prolongedsound")]
    XK_prolongedsound,
    /// XK_voicedsound
    #[strum(serialize = "voicedsound")]
    XK_voicedsound,
    /// XK_semivoicedsound
    #[strum(serialize = "semivoicedsound")]
    XK_semivoicedsound,
    /// XK_numerosign
    #[strum(serialize = "numerosign")]
    XK_numerosign,
    /// XK_leftradical
    #[strum(serialize = "leftradical")]
    XK_leftradical,
    /// XK_topleftradical
    #[strum(serialize = "topleftradical")]
    XK_topleftradical,
    /// XK_horizconnector
    #[strum(serialize = "horizconnector")]
    XK_horizconnector,
    /// XK_topintegral
    #[strum(serialize = "topintegral")]
    XK_topintegral,
    /// XK_botintegral
    #[strum(serialize = "botintegral")]
    XK_botintegral,
    /// XK_vertconnector
    #[strum(serialize = "vertconnector")]
    XK_vertconnector,
    /// XK_topleftsqbracket
    #[strum(serialize = "topleftsqbracket")]
    XK_topleftsqbracket,
    /// XK_botleftsqbracket
    #[strum(serialize = "botleftsqbracket")]
    XK_botleftsqbracket,
    /// XK_toprightsqbracket
    #[strum(serialize = "toprightsqbracket")]
    XK_toprightsqbracket,
    /// XK_botrightsqbracket
    #[strum(serialize = "botrightsqbracket")]
    XK_botrightsqbracket,
    /// XK_topleftparens
    #[strum(serialize = "topleftparens")]
    XK_topleftparens,
    /// XK_botleftparens
    #[strum(serialize = "botleftparens")]
    XK_botleftparens,
    /// XK_toprightparens
    #[strum(serialize = "toprightparens")]
    XK_toprightparens,
    /// XK_botrightparens
    #[strum(serialize = "botrightparens")]
    XK_botrightparens,
    /// XK_leftmiddlecurlybrace
    #[strum(serialize = "leftmiddlecurlybrace")]
    XK_leftmiddlecurlybrace,
    /// XK_rightmiddlecurlybrace
    #[strum(serialize = "rightmiddlecurlybrace")]
    XK_rightmiddlecurlybrace,
    /// XK_lessthanequal
    #[strum(serialize = "lessthanequal")]
    XK_lessthanequal,
    /// XK_notequal
    #[strum(serialize = "notequal")]
    XK_notequal,
    /// XK_greaterthanequal
    #[strum(serialize = "greaterthanequal")]
    XK_greaterthanequal,
    /// XK_integral
    #[strum(serialize = "integral")]
    XK_integral,
    /// XK_therefore
    #[strum(serialize = "therefore")]
    XK_therefore,
    /// XK_variation
    #[strum(serialize = "variation")]
    XK_variation,
    /// XK_infinity
    #[strum(serialize = "infinity")]
    XK_infinity,
    /// XK_nabla
    #[strum(serialize = "nabla")]
    XK_nabla,
    /// XK_approximate
    #[strum(serialize = "approximate")]
    XK_approximate,
    /// XK_similarequal
    #[strum(serialize = "similarequal")]
    XK_similarequal,
    /// XK_ifonlyif
    #[strum(serialize = "ifonlyif")]
    XK_ifonlyif,
    /// XK_implies
    #[strum(serialize = "implies")]
    XK_implies,
    /// XK_identical
    #[strum(serialize = "identical")]
    XK_identical,
    /// XK_radical
    #[strum(serialize = "radical")]
    XK_radical,
    /// XK_includedin
    #[strum(serialize = "includedin")]
    XK_includedin,
    /// XK_includes
    #[strum(serialize = "includes")]
    XK_includes,
    /// XK_intersection
    #[strum(serialize = "intersection")]
    XK_intersection,
    /// XK_union
    #[strum(serialize = "union")]
    XK_union,
    /// XK_logicaland
    #[strum(serialize = "logicaland")]
    XK_logicaland,
    /// XK_logicalor
    #[strum(serialize = "logicalor")]
    XK_logicalor,
    /// XK_partialderivative
    #[strum(serialize = "partialderivative")]
    XK_partialderivative,
    /// XK_function
    #[strum(serialize = "function")]
    XK_function,
    /// XK_leftarrow
    #[strum(serialize = "leftarrow")]
    XK_leftarrow,
    /// XK_uparrow
    #[strum(serialize = "uparrow")]
    XK_uparrow,
    /// XK_rightarrow
    #[strum(serialize = "rightarrow")]
    XK_rightarrow,
    /// XK_downarrow
    #[strum(serialize = "downarrow")]
    XK_downarrow,
    /// XK_blank
    #[strum(serialize = "blank")]
    XK_blank,
    /// XK_soliddiamond
    #[strum(serialize = "soliddiamond")]
    XK_soliddiamond,
    /// XK_checkerboard
    #[strum(serialize = "checkerboard")]
    XK_checkerboard,
    /// XK_ht
    #[strum(serialize = "ht")]
    XK_ht,
    /// XK_ff
    #[strum(serialize = "ff")]
    XK_ff,
    /// XK_cr
    #[strum(serialize = "cr")]
    XK_cr,
    /// XK_lf
    #[strum(serialize = "lf")]
    XK_lf,
    /// XK_nl
    #[strum(serialize = "nl")]
    XK_nl,
    /// XK_vt
    #[strum(serialize = "vt")]
    XK_vt,
    /// XK_lowrightcorner
    #[strum(serialize = "lowrightcorner")]
    XK_lowrightcorner,
    /// XK_uprightcorner
    #[strum(serialize = "uprightcorner")]
    XK_uprightcorner,
    /// XK_upleftcorner
    #[strum(serialize = "upleftcorner")]
    XK_upleftcorner,
    /// XK_lowleftcorner
    #[strum(serialize = "lowleftcorner")]
    XK_lowleftcorner,
    /// XK_crossinglines
    #[strum(serialize = "crossinglines")]
    XK_crossinglines,
    /// XK_leftt
    #[strum(serialize = "leftt")]
    XK_leftt,
    /// XK_rightt
    #[strum(serialize = "rightt")]
    XK_rightt,
    /// XK_bott
    #[strum(serialize = "bott")]
    XK_bott,
    /// XK_topt
    #[strum(serialize = "topt")]
    XK_topt,
    /// XK_vertbar
    #[strum(serialize = "vertbar")]
    XK_vertbar,
    /// XK_emspace
    #[strum(serialize = "emspace")]
    XK_emspace,
    /// XK_enspace
    #[strum(serialize = "enspace")]
    XK_enspace,
    /// XK_em3space
    #[strum(serialize = "em3space")]
    XK_em3space,
    /// XK_em4space
    #[strum(serialize = "em4space")]
    XK_em4space,
    /// XK_digitspace
    #[strum(serialize = "digitspace")]
    XK_digitspace,
    /// XK_punctspace
    #[strum(serialize = "punctspace")]
    XK_punctspace,
    /// XK_thinspace
    #[strum(serialize = "thinspace")]
    XK_thinspace,
    /// XK_hairspace
    #[strum(serialize = "hairspace")]
    XK_hairspace,
    /// XK_emdash
    #[strum(serialize = "emdash")]
    XK_emdash,
    /// XK_endash
    #[strum(serialize = "endash")]
    XK_endash,
    /// XK_signifblank
    #[strum(serialize = "signifblank")]
    XK_signifblank,
    /// XK_ellipsis
    #[strum(serialize = "ellipsis")]
    XK_ellipsis,
    /// XK_doubbaselinedot
    #[strum(serialize = "doubbaselinedot")]
    XK_doubbaselinedot,
    /// XK_onethird
    #[strum(serialize = "onethird")]
    XK_onethird,
    /// XK_twothirds
    #[strum(serialize = "twothirds")]
    XK_twothirds,
    /// XK_onefifth
    #[strum(serialize = "onefifth")]
    XK_onefifth,
    /// XK_twofifths
    #[strum(serialize = "twofifths")]
    XK_twofifths,
    /// XK_threefifths
    #[strum(serialize = "threefifths")]
    XK_threefifths,
    /// XK_fourfifths
    #[strum(serialize = "fourfifths")]
    XK_fourfifths,
    /// XK_onesixth
    #[strum(serialize = "onesixth")]
    XK_onesixth,
    /// XK_fivesixths
    #[strum(serialize = "fivesixths")]
    XK_fivesixths,
    /// XK_careof
    #[strum(serialize = "careof")]
    XK_careof,
    /// XK_figdash
    #[strum(serialize = "figdash")]
    XK_figdash,
    /// XK_leftanglebracket
    #[strum(serialize = "leftanglebracket")]
    XK_leftanglebracket,
    /// XK_decimalpoint
    #[strum(serialize = "decimalpoint")]
    XK_decimalpoint,
    /// XK_rightanglebracket
    #[strum(serialize = "rightanglebracket")]
    XK_rightanglebracket,
    /// XK_marker
    #[strum(serialize = "marker")]
    XK_marker,
    /// XK_oneeighth
    #[strum(serialize = "oneeighth")]
    XK_oneeighth,
    /// XK_threeeighths
    #[strum(serialize = "threeeighths")]
    XK_threeeighths,
    /// XK_fiveeighths
    #[strum(serialize = "fiveeighths")]
    XK_fiveeighths,
    /// XK_seveneighths
    #[strum(serialize = "seveneighths")]
    XK_seveneighths,
    /// XK_trademark
    #[strum(serialize = "trademark")]
    XK_trademark,
    /// XK_signaturemark
    #[strum(serialize = "signaturemark")]
    XK_signaturemark,
    /// XK_leftopentriangle
    #[strum(serialize = "leftopentriangle")]
    XK_leftopentriangle,
    /// XK_rightopentriangle
    #[strum(serialize = "rightopentriangle")]
    XK_rightopentriangle,
    /// XK_emopenrectangle
    #[strum(serialize = "emopenrectangle")]
    XK_emopenrectangle,
    /// XK_leftsinglequotemark
    #[strum(serialize = "leftsinglequotemark")]
    XK_leftsinglequotemark,
    /// XK_rightsinglequotemark
    #[strum(serialize = "rightsinglequotemark")]
    XK_rightsinglequotemark,
    /// XK_leftdoublequotemark
    #[strum(serialize = "leftdoublequotemark")]
    XK_leftdoublequotemark,
    /// XK_rightdoublequotemark
    #[strum(serialize = "rightdoublequotemark")]
    XK_rightdoublequotemark,
    /// XK_prescription
    #[strum(serialize = "prescription")]
    XK_prescription,
    /// XK_permille
    #[strum(serialize = "permille")]
    XK_permille,
    /// XK_minutes
    #[strum(serialize = "minutes")]
    XK_minutes,
    /// XK_seconds
    #[strum(serialize = "seconds")]
    XK_seconds,
    /// XK_latincross
    #[strum(serialize = "latincross")]
    XK_latincross,
    /// XK_hexagram
    #[strum(serialize = "hexagram")]
    XK_hexagram,
    /// XK_emfilledrect
    #[strum(serialize = "emfilledrect")]
    XK_emfilledrect,
    /// XK_openstar
    #[strum(serialize = "openstar")]
    XK_openstar,
    /// XK_leftpointer
    #[strum(serialize = "leftpointer")]
    XK_leftpointer,
    /// XK_rightpointer
    #[strum(serialize = "rightpointer")]
    XK_rightpointer,
    /// XK_club
    #[strum(serialize = "club")]
    XK_club,
    /// XK_diamond
    #[strum(serialize = "diamond")]
    XK_diamond,
    /// XK_heart
    #[strum(serialize = "heart")]
    XK_heart,
    /// XK_maltesecross
    #[strum(serialize = "maltesecross")]
    XK_maltesecross,
    /// XK_dagger
    #[strum(serialize = "dagger")]
    XK_dagger,
    /// XK_doubledagger
    #[strum(serialize = "doubledagger")]
    XK_doubledagger,
    /// XK_checkmark
    #[strum(serialize = "checkmark")]
    XK_checkmark,
    /// XK_ballotcross
    #[strum(serialize = "ballotcross")]
    XK_ballotcross,
    /// XK_musicalsharp
    #[strum(serialize = "musicalsharp")]
    XK_musicalsharp,
    /// XK_musicalflat
    #[strum(serialize = "musicalflat")]
    XK_musicalflat,
    /// XK_malesymbol
    #[strum(serialize = "malesymbol")]
    XK_malesymbol,
    /// XK_femalesymbol
    #[strum(serialize = "femalesymbol")]
    XK_femalesymbol,
    /// XK_telephone
    #[strum(serialize = "telephone")]
    XK_telephone,
    /// XK_telephonerecorder
    #[strum(serialize = "telephonerecorder")]
    XK_telephonerecorder,
    /// XK_phonographcopyright
    #[strum(serialize = "phonographcopyright")]
    XK_phonographcopyright,
    /// XK_caret
    #[strum(serialize = "caret")]
    XK_caret,
    /// XK_singlelowquotemark
    #[strum(serialize = "singlelowquotemark")]
    XK_singlelowquotemark,
    /// XK_doublelowquotemark
    #[strum(serialize = "doublelowquotemark")]
    XK_doublelowquotemark,
    /// XK_cursor
    #[strum(serialize = "cursor")]
    XK_cursor,
    /// XK_leftcaret
    #[strum(serialize = "leftcaret")]
    XK_leftcaret,
    /// XK_rightcaret
    #[strum(serialize = "rightcaret")]
    XK_rightcaret,
    /// XK_downcaret
    #[strum(serialize = "downcaret")]
    XK_downcaret,
    /// XK_upcaret
    #[strum(serialize = "upcaret")]
    XK_upcaret,
    /// XK_overbar
    #[strum(serialize = "overbar")]
    XK_overbar,
    /// XK_downtack
    #[strum(serialize = "downtack")]
    XK_downtack,
    /// XK_upshoe
    #[strum(serialize = "upshoe")]
    XK_upshoe,
    /// XK_downstile
    #[strum(serialize = "downstile")]
    XK_downstile,
    /// XK_underbar
    #[strum(serialize = "underbar")]
    XK_underbar,
    /// XK_jot
    #[strum(serialize = "jot")]
    XK_jot,
    /// XK_quad
    #[strum(serialize = "quad")]
    XK_quad,
    /// XK_uptack
    #[strum(serialize = "uptack")]
    XK_uptack,
    /// XK_upstile
    #[strum(serialize = "upstile")]
    XK_upstile,
    /// XK_downshoe
    #[strum(serialize = "downshoe")]
    XK_downshoe,
    /// XK_rightshoe
    #[strum(serialize = "rightshoe")]
    XK_rightshoe,
    /// XK_leftshoe
    #[strum(serialize = "leftshoe")]
    XK_leftshoe,
    /// XK_lefttack
    #[strum(serialize = "lefttack")]
    XK_lefttack,
    /// XK_righttack
    #[strum(serialize = "righttack")]
    XK_righttack,
    /// XK_Korean_Won
    #[strum(serialize = "Korean_Won")]
    XK_Korean_Won,
    /// XK_Ibreve
    #[strum(serialize = "Ibreve")]
    XK_Ibreve,
    /// XK_Zstroke
    #[strum(serialize = "Zstroke")]
    XK_Zstroke,
    /// XK_Gcaron
    #[strum(serialize = "Gcaron")]
    XK_Gcaron,
    /// XK_Ocaron
    #[strum(serialize = "Ocaron")]
    XK_Ocaron,
    /// XK_Obarred
    #[strum(serialize = "Obarred")]
    XK_Obarred,
    /// XK_ibreve
    #[strum(serialize = "ibreve")]
    XK_ibreve,
    /// XK_zstroke
    #[strum(serialize = "zstroke")]
    XK_zstroke,
    /// XK_gcaron
    #[strum(serialize = "gcaron")]
    XK_gcaron,
    /// XK_ocaron
    #[strum(serialize = "ocaron")]
    XK_ocaron,
    /// XK_obarred
    #[strum(serialize = "obarred")]
    XK_obarred,
    /// XK_SCHWA
    #[strum(serialize = "SCHWA")]
    XK_SCHWA,
    /// XK_schwa
    #[strum(serialize = "schwa")]
    XK_schwa,
    /// XK_EZH
    #[strum(serialize = "EZH")]
    XK_EZH,
    /// XK_ezh
    #[strum(serialize = "ezh")]
    XK_ezh,
    /// XK_Abreveacute
    #[strum(serialize = "Abreveacute")]
    XK_Abreveacute,
    /// XK_abreveacute
    #[strum(serialize = "abreveacute")]
    XK_abreveacute,
    /// XK_Abrevetilde
    #[strum(serialize = "Abrevetilde")]
    XK_Abrevetilde,
    /// XK_abrevetilde
    #[strum(serialize = "abrevetilde")]
    XK_abrevetilde,
    /// XK_Etilde
    #[strum(serialize = "Etilde")]
    XK_Etilde,
    /// XK_etilde
    #[strum(serialize = "etilde")]
    XK_etilde,
    /// XK_Ytilde
    #[strum(serialize = "Ytilde")]
    XK_Ytilde,
    /// XK_ytilde
    #[strum(serialize = "ytilde")]
    XK_ytilde,
    /// XK_EcuSign
    #[strum(serialize = "EcuSign")]
    XK_EcuSign,
    /// XK_ColonSign
    #[strum(serialize = "ColonSign")]
    XK_ColonSign,
    /// XK_CruzeiroSign
    #[strum(serialize = "CruzeiroSign")]
    XK_CruzeiroSign,
    /// XK_FFrancSign
    #[strum(serialize = "FFrancSign")]
    XK_FFrancSign,
    /// XK_LiraSign
    #[strum(serialize = "LiraSign")]
    XK_LiraSign,
    /// XK_MillSign
    #[strum(serialize = "MillSign")]
    XK_MillSign,
    /// XK_NairaSign
    #[strum(serialize = "NairaSign")]
    XK_NairaSign,
    /// XK_PesetaSign
    #[strum(serialize = "PesetaSign")]
    XK_PesetaSign,
    /// XK_RupeeSign
    #[strum(serialize = "RupeeSign")]
    XK_RupeeSign,
    /// XK_WonSign
    #[strum(serialize = "WonSign")]
    XK_WonSign,
    /// XK_NewSheqelSign
    #[strum(serialize = "NewSheqelSign")]
    XK_NewSheqelSign,
    /// XK_DongSign
    #[strum(serialize = "DongSign")]
    XK_DongSign,
    /// XK_EuroSign
    #[strum(serialize = "EuroSign")]
    XK_EuroSign,

    /// XF86XK_MonBrightnessUp
    #[strum(serialize = "XF86MonBrightnessUp")]
    XF86XK_MonBrightnessUp,

    /// XF86XK_MonBrightnessDown
    #[strum(serialize = "XF86MonBrightnessDown")]
    XF86XK_MonBrightnessDown,

    /// XF86XK_KbdLightOnOff
    #[strum(serialize = "XF86KbdLightOnOff")]
    XF86XK_KbdLightOnOff,

    /// XF86XK_KbdBrightnessUp
    #[strum(serialize = "XF86KbdBrightnessUp")]
    XF86XK_KbdBrightnessUp,

    /// XF86XK_KbdBrightnessDown
    #[strum(serialize = "XF86KbdBrightnessDown")]
    XF86XK_KbdBrightnessDown,

    /// XF86XK_MonBrightnessCycle
    #[strum(serialize = "XF86MonBrightnessCycle")]
    XF86XK_MonBrightnessCycle,

    /// XF86XK_Standby
    #[strum(serialize = "XF86Standby")]
    XF86XK_Standby,

    /// XF86XK_AudioLowerVolume
    #[strum(serialize = "XF86AudioLowerVolume")]
    XF86XK_AudioLowerVolume,
    /// XF86XK_AudioMute
    #[strum(serialize = "XF86AudioMute")]
    XF86XK_AudioMute,
    /// XF86XK_AudioRaiseVolume
    #[strum(serialize = "XF86AudioRaiseVolume")]
    XF86XK_AudioRaiseVolume,
    /// XF86XK_AudioPlay
    #[strum(serialize = "XF86AudioPlay")]
    XF86XK_AudioPlay,
    /// XF86XK_AudioStop
    #[strum(serialize = "XF86AudioStop")]
    XF86XK_AudioStop,
    /// XF86XK_AudioPrev
    #[strum(serialize = "XF86AudioPrev")]
    XF86XK_AudioPrev,
    /// XF86XK_AudioNext
    #[strum(serialize = "XF86AudioNext")]
    XF86XK_AudioNext,
}

impl XKeySym {
    /// Convert this keysym to its utf8 representation if possible
    pub fn as_utf8_string(&self) -> Result<String, std::string::FromUtf8Error> {
        Ok(String::from_utf8(
            (match self {
                XKeySym::XK_BackSpace => 0xff08,
                XKeySym::XK_Tab => 0xff09,
                XKeySym::XK_Linefeed => 0xff0a,
                XKeySym::XK_Clear => 0xff0b,
                XKeySym::XK_Return => 0xff0d,
                XKeySym::XK_Pause => 0xff13,
                XKeySym::XK_Scroll_Lock => 0xff14,
                XKeySym::XK_Sys_Req => 0xff15,
                XKeySym::XK_Escape => 0xff1b,
                XKeySym::XK_Delete => 0xffff,
                XKeySym::XK_Home => 0xff50,
                XKeySym::XK_Left => 0xff51,
                XKeySym::XK_Up => 0xff52,
                XKeySym::XK_Right => 0xff53,
                XKeySym::XK_Down => 0xff54,
                XKeySym::XK_Prior => 0xff55,
                XKeySym::XK_Page_Up => 0xff55,
                XKeySym::XK_Next => 0xff56,
                XKeySym::XK_Page_Down => 0xff56,
                XKeySym::XK_End => 0xff57,
                XKeySym::XK_Begin => 0xff58,
                XKeySym::XK_Select => 0xff60,
                XKeySym::XK_Print => 0xff61,
                XKeySym::XK_Execute => 0xff62,
                XKeySym::XK_Insert => 0xff63,
                XKeySym::XK_Undo => 0xff65,
                XKeySym::XK_Redo => 0xff66,
                XKeySym::XK_Menu => 0xff67,
                XKeySym::XK_Find => 0xff68,
                XKeySym::XK_Cancel => 0xff69,
                XKeySym::XK_Help => 0xff6a,
                XKeySym::XK_Break => 0xff6b,
                XKeySym::XK_Mode_switch => 0xff7e,
                XKeySym::XK_script_switch => 0xff7e,
                XKeySym::XK_Num_Lock => 0xff7f,
                XKeySym::XK_KP_Space => 0xff80,
                XKeySym::XK_KP_Tab => 0xff89,
                XKeySym::XK_KP_Enter => 0xff8d,
                XKeySym::XK_KP_F1 => 0xff91,
                XKeySym::XK_KP_F2 => 0xff92,
                XKeySym::XK_KP_F3 => 0xff93,
                XKeySym::XK_KP_F4 => 0xff94,
                XKeySym::XK_KP_Home => 0xff95,
                XKeySym::XK_KP_Left => 0xff96,
                XKeySym::XK_KP_Up => 0xff97,
                XKeySym::XK_KP_Right => 0xff98,
                XKeySym::XK_KP_Down => 0xff99,
                XKeySym::XK_KP_Prior => 0xff9a,
                XKeySym::XK_KP_Page_Up => 0xff9a,
                XKeySym::XK_KP_Next => 0xff9b,
                XKeySym::XK_KP_Page_Down => 0xff9b,
                XKeySym::XK_KP_End => 0xff9c,
                XKeySym::XK_KP_Begin => 0xff9d,
                XKeySym::XK_KP_Insert => 0xff9e,
                XKeySym::XK_KP_Delete => 0xff9f,
                XKeySym::XK_KP_Equal => 0xffbd,
                XKeySym::XK_KP_Multiply => 0xffaa,
                XKeySym::XK_KP_Add => 0xffab,
                XKeySym::XK_KP_Separator => 0xffac,
                XKeySym::XK_KP_Subtract => 0xffad,
                XKeySym::XK_KP_Decimal => 0xffae,
                XKeySym::XK_KP_Divide => 0xffaf,
                XKeySym::XK_KP_0 => 0xffb0,
                XKeySym::XK_KP_1 => 0xffb1,
                XKeySym::XK_KP_2 => 0xffb2,
                XKeySym::XK_KP_3 => 0xffb3,
                XKeySym::XK_KP_4 => 0xffb4,
                XKeySym::XK_KP_5 => 0xffb5,
                XKeySym::XK_KP_6 => 0xffb6,
                XKeySym::XK_KP_7 => 0xffb7,
                XKeySym::XK_KP_8 => 0xffb8,
                XKeySym::XK_KP_9 => 0xffb9,
                XKeySym::XK_F1 => 0xffbe,
                XKeySym::XK_F2 => 0xffbf,
                XKeySym::XK_F3 => 0xffc0,
                XKeySym::XK_F4 => 0xffc1,
                XKeySym::XK_F5 => 0xffc2,
                XKeySym::XK_F6 => 0xffc3,
                XKeySym::XK_F7 => 0xffc4,
                XKeySym::XK_F8 => 0xffc5,
                XKeySym::XK_F9 => 0xffc6,
                XKeySym::XK_F10 => 0xffc7,
                XKeySym::XK_F11 => 0xffc8,
                XKeySym::XK_L1 => 0xffc8,
                XKeySym::XK_F12 => 0xffc9,
                XKeySym::XK_L2 => 0xffc9,
                XKeySym::XK_F13 => 0xffca,
                XKeySym::XK_L3 => 0xffca,
                XKeySym::XK_F14 => 0xffcb,
                XKeySym::XK_L4 => 0xffcb,
                XKeySym::XK_F15 => 0xffcc,
                XKeySym::XK_L5 => 0xffcc,
                XKeySym::XK_F16 => 0xffcd,
                XKeySym::XK_L6 => 0xffcd,
                XKeySym::XK_F17 => 0xffce,
                XKeySym::XK_L7 => 0xffce,
                XKeySym::XK_F18 => 0xffcf,
                XKeySym::XK_L8 => 0xffcf,
                XKeySym::XK_F19 => 0xffd0,
                XKeySym::XK_L9 => 0xffd0,
                XKeySym::XK_F20 => 0xffd1,
                XKeySym::XK_L10 => 0xffd1,
                XKeySym::XK_F21 => 0xffd2,
                XKeySym::XK_R1 => 0xffd2,
                XKeySym::XK_F22 => 0xffd3,
                XKeySym::XK_R2 => 0xffd3,
                XKeySym::XK_F23 => 0xffd4,
                XKeySym::XK_R3 => 0xffd4,
                XKeySym::XK_F24 => 0xffd5,
                XKeySym::XK_R4 => 0xffd5,
                XKeySym::XK_F25 => 0xffd6,
                XKeySym::XK_R5 => 0xffd6,
                XKeySym::XK_F26 => 0xffd7,
                XKeySym::XK_R6 => 0xffd7,
                XKeySym::XK_F27 => 0xffd8,
                XKeySym::XK_R7 => 0xffd8,
                XKeySym::XK_F28 => 0xffd9,
                XKeySym::XK_R8 => 0xffd9,
                XKeySym::XK_F29 => 0xffda,
                XKeySym::XK_R9 => 0xffda,
                XKeySym::XK_F30 => 0xffdb,
                XKeySym::XK_R10 => 0xffdb,
                XKeySym::XK_F31 => 0xffdc,
                XKeySym::XK_R11 => 0xffdc,
                XKeySym::XK_F32 => 0xffdd,
                XKeySym::XK_R12 => 0xffdd,
                XKeySym::XK_F33 => 0xffde,
                XKeySym::XK_R13 => 0xffde,
                XKeySym::XK_F34 => 0xffdf,
                XKeySym::XK_R14 => 0xffdf,
                XKeySym::XK_F35 => 0xffe0,
                XKeySym::XK_R15 => 0xffe0,
                XKeySym::XK_Shift_L => 0xffe1,
                XKeySym::XK_Shift_R => 0xffe2,
                XKeySym::XK_Control_L => 0xffe3,
                XKeySym::XK_Control_R => 0xffe4,
                XKeySym::XK_Caps_Lock => 0xffe5,
                XKeySym::XK_Shift_Lock => 0xffe6,
                XKeySym::XK_Meta_L => 0xffe7,
                XKeySym::XK_Meta_R => 0xffe8,
                XKeySym::XK_Alt_L => 0xffe9,
                XKeySym::XK_Alt_R => 0xffea,
                XKeySym::XK_Super_L => 0xffeb,
                XKeySym::XK_Super_R => 0xffec,
                XKeySym::XK_Hyper_L => 0xffed,
                XKeySym::XK_Hyper_R => 0xffee,
                XKeySym::XK_ISO_Lock => 0xfe01,
                XKeySym::XK_ISO_Level2_Latch => 0xfe02,
                XKeySym::XK_ISO_Level3_Shift => 0xfe03,
                XKeySym::XK_ISO_Level3_Latch => 0xfe04,
                XKeySym::XK_ISO_Level3_Lock => 0xfe05,
                XKeySym::XK_ISO_Level5_Shift => 0xfe11,
                XKeySym::XK_ISO_Level5_Latch => 0xfe12,
                XKeySym::XK_ISO_Level5_Lock => 0xfe13,
                XKeySym::XK_ISO_Left_Tab => 0xfe20,
                XKeySym::XK_ISO_Partial_Space_Left => 0xfe25,
                XKeySym::XK_ISO_Partial_Space_Right => 0xfe26,
                XKeySym::XK_ISO_Set_Margin_Left => 0xfe27,
                XKeySym::XK_ISO_Set_Margin_Right => 0xfe28,
                XKeySym::XK_ISO_Continuous_Underline => 0xfe30,
                XKeySym::XK_ISO_Discontinuous_Underline => 0xfe31,
                XKeySym::XK_ISO_Emphasize => 0xfe32,
                XKeySym::XK_ISO_Center_Object => 0xfe33,
                XKeySym::XK_ISO_Enter => 0xfe34,
                XKeySym::XK_Terminate_Server => 0xfed5,
                XKeySym::XK_ch => 0xfea0,
                XKeySym::XK_Ch => 0xfea1,
                XKeySym::XK_CH => 0xfea2,
                XKeySym::XK_c_h => 0xfea3,
                XKeySym::XK_C_h => 0xfea4,
                XKeySym::XK_C_H => 0xfea5,
                XKeySym::XK_3270_Duplicate => 0xfd01,
                XKeySym::XK_3270_FieldMark => 0xfd02,
                XKeySym::XK_3270_Right2 => 0xfd03,
                XKeySym::XK_3270_Left2 => 0xfd04,
                XKeySym::XK_3270_BackTab => 0xfd05,
                XKeySym::XK_3270_EraseEOF => 0xfd06,
                XKeySym::XK_3270_EraseInput => 0xfd07,
                XKeySym::XK_3270_Reset => 0xfd08,
                XKeySym::XK_3270_Quit => 0xfd09,
                XKeySym::XK_3270_PA1 => 0xfd0a,
                XKeySym::XK_3270_PA2 => 0xfd0b,
                XKeySym::XK_3270_PA3 => 0xfd0c,
                XKeySym::XK_3270_Test => 0xfd0d,
                XKeySym::XK_3270_Attn => 0xfd0e,
                XKeySym::XK_3270_CursorBlink => 0xfd0f,
                XKeySym::XK_3270_AltCursor => 0xfd10,
                XKeySym::XK_3270_KeyClick => 0xfd11,
                XKeySym::XK_3270_Jump => 0xfd12,
                XKeySym::XK_3270_Ident => 0xfd13,
                XKeySym::XK_3270_Rule => 0xfd14,
                XKeySym::XK_3270_Copy => 0xfd15,
                XKeySym::XK_3270_Play => 0xfd16,
                XKeySym::XK_3270_Setup => 0xfd17,
                XKeySym::XK_3270_Record => 0xfd18,
                XKeySym::XK_3270_DeleteWord => 0xfd1a,
                XKeySym::XK_3270_ExSelect => 0xfd1b,
                XKeySym::XK_3270_CursorSelect => 0xfd1c,
                XKeySym::XK_3270_Enter => 0xfd1e,
                XKeySym::XK_space => 0x0020,
                XKeySym::XK_exclam => 0x0021,
                XKeySym::XK_quotedbl => 0x0022,
                XKeySym::XK_numbersign => 0x0023,
                XKeySym::XK_dollar => 0x0024,
                XKeySym::XK_percent => 0x0025,
                XKeySym::XK_ampersand => 0x0026,
                XKeySym::XK_apostrophe => 0x0027,
                XKeySym::XK_quoteright => 0x0027,
                XKeySym::XK_parenleft => 0x0028,
                XKeySym::XK_parenright => 0x0029,
                XKeySym::XK_asterisk => 0x002a,
                XKeySym::XK_plus => 0x002b,
                XKeySym::XK_comma => 0x002c,
                XKeySym::XK_minus => 0x002d,
                XKeySym::XK_period => 0x002e,
                XKeySym::XK_slash => 0x002f,
                XKeySym::XK_0 => 0x0030,
                XKeySym::XK_1 => 0x0031,
                XKeySym::XK_2 => 0x0032,
                XKeySym::XK_3 => 0x0033,
                XKeySym::XK_4 => 0x0034,
                XKeySym::XK_5 => 0x0035,
                XKeySym::XK_6 => 0x0036,
                XKeySym::XK_7 => 0x0037,
                XKeySym::XK_8 => 0x0038,
                XKeySym::XK_9 => 0x0039,
                XKeySym::XK_colon => 0x003a,
                XKeySym::XK_semicolon => 0x003b,
                XKeySym::XK_less => 0x003c,
                XKeySym::XK_equal => 0x003d,
                XKeySym::XK_greater => 0x003e,
                XKeySym::XK_question => 0x003f,
                XKeySym::XK_at => 0x0040,
                XKeySym::XK_A => 0x0041,
                XKeySym::XK_B => 0x0042,
                XKeySym::XK_C => 0x0043,
                XKeySym::XK_D => 0x0044,
                XKeySym::XK_E => 0x0045,
                XKeySym::XK_F => 0x0046,
                XKeySym::XK_G => 0x0047,
                XKeySym::XK_H => 0x0048,
                XKeySym::XK_I => 0x0049,
                XKeySym::XK_J => 0x004a,
                XKeySym::XK_K => 0x004b,
                XKeySym::XK_L => 0x004c,
                XKeySym::XK_M => 0x004d,
                XKeySym::XK_N => 0x004e,
                XKeySym::XK_O => 0x004f,
                XKeySym::XK_P => 0x0050,
                XKeySym::XK_Q => 0x0051,
                XKeySym::XK_R => 0x0052,
                XKeySym::XK_S => 0x0053,
                XKeySym::XK_T => 0x0054,
                XKeySym::XK_U => 0x0055,
                XKeySym::XK_V => 0x0056,
                XKeySym::XK_W => 0x0057,
                XKeySym::XK_X => 0x0058,
                XKeySym::XK_Y => 0x0059,
                XKeySym::XK_Z => 0x005a,
                XKeySym::XK_bracketleft => 0x005b,
                XKeySym::XK_backslash => 0x005c,
                XKeySym::XK_bracketright => 0x005d,
                XKeySym::XK_asciicircum => 0x005e,
                XKeySym::XK_underscore => 0x005f,
                XKeySym::XK_grave => 0x0060,
                XKeySym::XK_quoteleft => 0x0060,
                XKeySym::XK_a => 0x0061,
                XKeySym::XK_b => 0x0062,
                XKeySym::XK_c => 0x0063,
                XKeySym::XK_d => 0x0064,
                XKeySym::XK_e => 0x0065,
                XKeySym::XK_f => 0x0066,
                XKeySym::XK_g => 0x0067,
                XKeySym::XK_h => 0x0068,
                XKeySym::XK_i => 0x0069,
                XKeySym::XK_j => 0x006a,
                XKeySym::XK_k => 0x006b,
                XKeySym::XK_l => 0x006c,
                XKeySym::XK_m => 0x006d,
                XKeySym::XK_n => 0x006e,
                XKeySym::XK_o => 0x006f,
                XKeySym::XK_p => 0x0070,
                XKeySym::XK_q => 0x0071,
                XKeySym::XK_r => 0x0072,
                XKeySym::XK_s => 0x0073,
                XKeySym::XK_t => 0x0074,
                XKeySym::XK_u => 0x0075,
                XKeySym::XK_v => 0x0076,
                XKeySym::XK_w => 0x0077,
                XKeySym::XK_x => 0x0078,
                XKeySym::XK_y => 0x0079,
                XKeySym::XK_z => 0x007a,
                XKeySym::XK_braceleft => 0x007b,
                XKeySym::XK_bar => 0x007c,
                XKeySym::XK_braceright => 0x007d,
                XKeySym::XK_asciitilde => 0x007e,
                XKeySym::XK_nobreakspace => 0x00a0,
                XKeySym::XK_exclamdown => 0x00a1,
                XKeySym::XK_cent => 0x00a2,
                XKeySym::XK_sterling => 0x00a3,
                XKeySym::XK_currency => 0x00a4,
                XKeySym::XK_yen => 0x00a5,
                XKeySym::XK_brokenbar => 0x00a6,
                XKeySym::XK_section => 0x00a7,
                XKeySym::XK_diaeresis => 0x00a8,
                XKeySym::XK_copyright => 0x00a9,
                XKeySym::XK_ordfeminine => 0x00aa,
                XKeySym::XK_guillemotleft => 0x00ab,
                XKeySym::XK_notsign => 0x00ac,
                XKeySym::XK_hyphen => 0x00ad,
                XKeySym::XK_registered => 0x00ae,
                XKeySym::XK_macron => 0x00af,
                XKeySym::XK_degree => 0x00b0,
                XKeySym::XK_plusminus => 0x00b1,
                XKeySym::XK_acute => 0x00b4,
                XKeySym::XK_mu => 0x00b5,
                XKeySym::XK_paragraph => 0x00b6,
                XKeySym::XK_periodcentered => 0x00b7,
                XKeySym::XK_cedilla => 0x00b8,
                XKeySym::XK_masculine => 0x00ba,
                XKeySym::XK_guillemotright => 0x00bb,
                XKeySym::XK_onequarter => 0x00bc,
                XKeySym::XK_onehalf => 0x00bd,
                XKeySym::XK_threequarters => 0x00be,
                XKeySym::XK_questiondown => 0x00bf,
                XKeySym::XK_Aacute => 0x00c1,
                XKeySym::XK_Atilde => 0x00c3,
                XKeySym::XK_Adiaeresis => 0x00c4,
                XKeySym::XK_Aring => 0x00c5,
                XKeySym::XK_AE => 0x00c6,
                XKeySym::XK_Ccedilla => 0x00c7,
                XKeySym::XK_Eacute => 0x00c9,
                XKeySym::XK_Ediaeresis => 0x00cb,
                XKeySym::XK_Iacute => 0x00cd,
                XKeySym::XK_Idiaeresis => 0x00cf,
                XKeySym::XK_ETH => 0x00d0,
                XKeySym::XK_Eth => 0x00d0,
                XKeySym::XK_Ntilde => 0x00d1,
                XKeySym::XK_Oacute => 0x00d3,
                XKeySym::XK_Otilde => 0x00d5,
                XKeySym::XK_Odiaeresis => 0x00d6,
                XKeySym::XK_multiply => 0x00d7,
                XKeySym::XK_Oslash => 0x00d8,
                XKeySym::XK_Ooblique => 0x00d8,
                XKeySym::XK_Uacute => 0x00da,
                XKeySym::XK_Udiaeresis => 0x00dc,
                XKeySym::XK_Yacute => 0x00dd,
                XKeySym::XK_ssharp => 0x00df,
                XKeySym::XK_aacute => 0x00e1,
                XKeySym::XK_atilde => 0x00e3,
                XKeySym::XK_adiaeresis => 0x00e4,
                XKeySym::XK_aring => 0x00e5,
                XKeySym::XK_ae => 0x00e6,
                XKeySym::XK_ccedilla => 0x00e7,
                XKeySym::XK_eacute => 0x00e9,
                XKeySym::XK_ediaeresis => 0x00eb,
                XKeySym::XK_iacute => 0x00ed,
                XKeySym::XK_idiaeresis => 0x00ef,
                XKeySym::XK_eth => 0x00f0,
                XKeySym::XK_ntilde => 0x00f1,
                XKeySym::XK_oacute => 0x00f3,
                XKeySym::XK_otilde => 0x00f5,
                XKeySym::XK_odiaeresis => 0x00f6,
                XKeySym::XK_division => 0x00f7,
                XKeySym::XK_oslash => 0x00f8,
                XKeySym::XK_ooblique => 0x00f8,
                XKeySym::XK_uacute => 0x00fa,
                XKeySym::XK_udiaeresis => 0x00fc,
                XKeySym::XK_yacute => 0x00fd,
                XKeySym::XK_ydiaeresis => 0x00ff,
                XKeySym::XK_Aogonek => 0x01a1,
                XKeySym::XK_breve => 0x01a2,
                XKeySym::XK_Lstroke => 0x01a3,
                XKeySym::XK_Lcaron => 0x01a5,
                XKeySym::XK_Sacute => 0x01a6,
                XKeySym::XK_Scaron => 0x01a9,
                XKeySym::XK_Scedilla => 0x01aa,
                XKeySym::XK_Tcaron => 0x01ab,
                XKeySym::XK_Zacute => 0x01ac,
                XKeySym::XK_Zcaron => 0x01ae,
                XKeySym::XK_aogonek => 0x01b1,
                XKeySym::XK_ogonek => 0x01b2,
                XKeySym::XK_lstroke => 0x01b3,
                XKeySym::XK_lcaron => 0x01b5,
                XKeySym::XK_sacute => 0x01b6,
                XKeySym::XK_caron => 0x01b7,
                XKeySym::XK_scaron => 0x01b9,
                XKeySym::XK_scedilla => 0x01ba,
                XKeySym::XK_tcaron => 0x01bb,
                XKeySym::XK_zacute => 0x01bc,
                XKeySym::XK_doubleacute => 0x01bd,
                XKeySym::XK_zcaron => 0x01be,
                XKeySym::XK_Racute => 0x01c0,
                XKeySym::XK_Abreve => 0x01c3,
                XKeySym::XK_Lacute => 0x01c5,
                XKeySym::XK_Cacute => 0x01c6,
                XKeySym::XK_Ccaron => 0x01c8,
                XKeySym::XK_Eogonek => 0x01ca,
                XKeySym::XK_Ecaron => 0x01cc,
                XKeySym::XK_Dcaron => 0x01cf,
                XKeySym::XK_Dstroke => 0x01d0,
                XKeySym::XK_Nacute => 0x01d1,
                XKeySym::XK_Ncaron => 0x01d2,
                XKeySym::XK_Odoubleacute => 0x01d5,
                XKeySym::XK_Rcaron => 0x01d8,
                XKeySym::XK_Uring => 0x01d9,
                XKeySym::XK_Udoubleacute => 0x01db,
                XKeySym::XK_Tcedilla => 0x01de,
                XKeySym::XK_racute => 0x01e0,
                XKeySym::XK_abreve => 0x01e3,
                XKeySym::XK_lacute => 0x01e5,
                XKeySym::XK_cacute => 0x01e6,
                XKeySym::XK_ccaron => 0x01e8,
                XKeySym::XK_eogonek => 0x01ea,
                XKeySym::XK_ecaron => 0x01ec,
                XKeySym::XK_dcaron => 0x01ef,
                XKeySym::XK_dstroke => 0x01f0,
                XKeySym::XK_nacute => 0x01f1,
                XKeySym::XK_ncaron => 0x01f2,
                XKeySym::XK_odoubleacute => 0x01f5,
                XKeySym::XK_rcaron => 0x01f8,
                XKeySym::XK_uring => 0x01f9,
                XKeySym::XK_udoubleacute => 0x01fb,
                XKeySym::XK_tcedilla => 0x01fe,
                XKeySym::XK_Hstroke => 0x02a1,
                XKeySym::XK_Gbreve => 0x02ab,
                XKeySym::XK_hstroke => 0x02b1,
                XKeySym::XK_idotless => 0x02b9,
                XKeySym::XK_gbreve => 0x02bb,
                XKeySym::XK_Ubreve => 0x02dd,
                XKeySym::XK_ubreve => 0x02fd,
                XKeySym::XK_kra => 0x03a2,
                XKeySym::XK_kappa => 0x03a2,
                XKeySym::XK_Rcedilla => 0x03a3,
                XKeySym::XK_Itilde => 0x03a5,
                XKeySym::XK_Lcedilla => 0x03a6,
                XKeySym::XK_Emacron => 0x03aa,
                XKeySym::XK_Gcedilla => 0x03ab,
                XKeySym::XK_Tslash => 0x03ac,
                XKeySym::XK_rcedilla => 0x03b3,
                XKeySym::XK_itilde => 0x03b5,
                XKeySym::XK_lcedilla => 0x03b6,
                XKeySym::XK_emacron => 0x03ba,
                XKeySym::XK_gcedilla => 0x03bb,
                XKeySym::XK_tslash => 0x03bc,
                XKeySym::XK_ENG => 0x03bd,
                XKeySym::XK_eng => 0x03bf,
                XKeySym::XK_Amacron => 0x03c0,
                XKeySym::XK_Iogonek => 0x03c7,
                XKeySym::XK_Imacron => 0x03cf,
                XKeySym::XK_Ncedilla => 0x03d1,
                XKeySym::XK_Omacron => 0x03d2,
                XKeySym::XK_Kcedilla => 0x03d3,
                XKeySym::XK_Uogonek => 0x03d9,
                XKeySym::XK_Utilde => 0x03dd,
                XKeySym::XK_Umacron => 0x03de,
                XKeySym::XK_amacron => 0x03e0,
                XKeySym::XK_iogonek => 0x03e7,
                XKeySym::XK_imacron => 0x03ef,
                XKeySym::XK_ncedilla => 0x03f1,
                XKeySym::XK_omacron => 0x03f2,
                XKeySym::XK_kcedilla => 0x03f3,
                XKeySym::XK_uogonek => 0x03f9,
                XKeySym::XK_utilde => 0x03fd,
                XKeySym::XK_umacron => 0x03fe,
                XKeySym::XK_Wacute => 0x1001e82,
                XKeySym::XK_wacute => 0x1001e83,
                XKeySym::XK_Wdiaeresis => 0x1001e84,
                XKeySym::XK_wdiaeresis => 0x1001e85,
                XKeySym::XK_OE => 0x13bc,
                XKeySym::XK_oe => 0x13bd,
                XKeySym::XK_Ydiaeresis => 0x13be,
                XKeySym::XK_overline => 0x047e,
                XKeySym::XK_prolongedsound => 0x04b0,
                XKeySym::XK_voicedsound => 0x04de,
                XKeySym::XK_semivoicedsound => 0x04df,
                XKeySym::XK_numerosign => 0x06b0,
                XKeySym::XK_leftradical => 0x08a1,
                XKeySym::XK_topleftradical => 0x08a2,
                XKeySym::XK_horizconnector => 0x08a3,
                XKeySym::XK_topintegral => 0x08a4,
                XKeySym::XK_botintegral => 0x08a5,
                XKeySym::XK_vertconnector => 0x08a6,
                XKeySym::XK_topleftsqbracket => 0x08a7,
                XKeySym::XK_botleftsqbracket => 0x08a8,
                XKeySym::XK_toprightsqbracket => 0x08a9,
                XKeySym::XK_botrightsqbracket => 0x08aa,
                XKeySym::XK_topleftparens => 0x08ab,
                XKeySym::XK_botleftparens => 0x08ac,
                XKeySym::XK_toprightparens => 0x08ad,
                XKeySym::XK_botrightparens => 0x08ae,
                XKeySym::XK_leftmiddlecurlybrace => 0x08af,
                XKeySym::XK_rightmiddlecurlybrace => 0x08b0,
                XKeySym::XK_lessthanequal => 0x08bc,
                XKeySym::XK_notequal => 0x08bd,
                XKeySym::XK_greaterthanequal => 0x08be,
                XKeySym::XK_integral => 0x08bf,
                XKeySym::XK_therefore => 0x08c0,
                XKeySym::XK_variation => 0x08c1,
                XKeySym::XK_infinity => 0x08c2,
                XKeySym::XK_nabla => 0x08c5,
                XKeySym::XK_approximate => 0x08c8,
                XKeySym::XK_similarequal => 0x08c9,
                XKeySym::XK_ifonlyif => 0x08cd,
                XKeySym::XK_implies => 0x08ce,
                XKeySym::XK_identical => 0x08cf,
                XKeySym::XK_radical => 0x08d6,
                XKeySym::XK_includedin => 0x08da,
                XKeySym::XK_includes => 0x08db,
                XKeySym::XK_intersection => 0x08dc,
                XKeySym::XK_union => 0x08dd,
                XKeySym::XK_logicaland => 0x08de,
                XKeySym::XK_logicalor => 0x08df,
                XKeySym::XK_partialderivative => 0x08ef,
                XKeySym::XK_function => 0x08f6,
                XKeySym::XK_leftarrow => 0x08fb,
                XKeySym::XK_uparrow => 0x08fc,
                XKeySym::XK_rightarrow => 0x08fd,
                XKeySym::XK_downarrow => 0x08fe,
                XKeySym::XK_blank => 0x09df,
                XKeySym::XK_soliddiamond => 0x09e0,
                XKeySym::XK_checkerboard => 0x09e1,
                XKeySym::XK_ht => 0x09e2,
                XKeySym::XK_ff => 0x09e3,
                XKeySym::XK_cr => 0x09e4,
                XKeySym::XK_lf => 0x09e5,
                XKeySym::XK_nl => 0x09e8,
                XKeySym::XK_vt => 0x09e9,
                XKeySym::XK_lowrightcorner => 0x09ea,
                XKeySym::XK_uprightcorner => 0x09eb,
                XKeySym::XK_upleftcorner => 0x09ec,
                XKeySym::XK_lowleftcorner => 0x09ed,
                XKeySym::XK_crossinglines => 0x09ee,
                XKeySym::XK_leftt => 0x09f4,
                XKeySym::XK_rightt => 0x09f5,
                XKeySym::XK_bott => 0x09f6,
                XKeySym::XK_topt => 0x09f7,
                XKeySym::XK_vertbar => 0x09f8,
                XKeySym::XK_emspace => 0x0aa1,
                XKeySym::XK_enspace => 0x0aa2,
                XKeySym::XK_em3space => 0x0aa3,
                XKeySym::XK_em4space => 0x0aa4,
                XKeySym::XK_digitspace => 0x0aa5,
                XKeySym::XK_punctspace => 0x0aa6,
                XKeySym::XK_thinspace => 0x0aa7,
                XKeySym::XK_hairspace => 0x0aa8,
                XKeySym::XK_emdash => 0x0aa9,
                XKeySym::XK_endash => 0x0aaa,
                XKeySym::XK_signifblank => 0x0aac,
                XKeySym::XK_ellipsis => 0x0aae,
                XKeySym::XK_doubbaselinedot => 0x0aaf,
                XKeySym::XK_onethird => 0x0ab0,
                XKeySym::XK_twothirds => 0x0ab1,
                XKeySym::XK_onefifth => 0x0ab2,
                XKeySym::XK_twofifths => 0x0ab3,
                XKeySym::XK_threefifths => 0x0ab4,
                XKeySym::XK_fourfifths => 0x0ab5,
                XKeySym::XK_onesixth => 0x0ab6,
                XKeySym::XK_fivesixths => 0x0ab7,
                XKeySym::XK_careof => 0x0ab8,
                XKeySym::XK_figdash => 0x0abb,
                XKeySym::XK_leftanglebracket => 0x0abc,
                XKeySym::XK_decimalpoint => 0x0abd,
                XKeySym::XK_rightanglebracket => 0x0abe,
                XKeySym::XK_marker => 0x0abf,
                XKeySym::XK_oneeighth => 0x0ac3,
                XKeySym::XK_threeeighths => 0x0ac4,
                XKeySym::XK_fiveeighths => 0x0ac5,
                XKeySym::XK_seveneighths => 0x0ac6,
                XKeySym::XK_trademark => 0x0ac9,
                XKeySym::XK_signaturemark => 0x0aca,
                XKeySym::XK_leftopentriangle => 0x0acc,
                XKeySym::XK_rightopentriangle => 0x0acd,
                XKeySym::XK_emopenrectangle => 0x0acf,
                XKeySym::XK_leftsinglequotemark => 0x0ad0,
                XKeySym::XK_rightsinglequotemark => 0x0ad1,
                XKeySym::XK_leftdoublequotemark => 0x0ad2,
                XKeySym::XK_rightdoublequotemark => 0x0ad3,
                XKeySym::XK_prescription => 0x0ad4,
                XKeySym::XK_permille => 0x0ad5,
                XKeySym::XK_minutes => 0x0ad6,
                XKeySym::XK_seconds => 0x0ad7,
                XKeySym::XK_latincross => 0x0ad9,
                XKeySym::XK_hexagram => 0x0ada,
                XKeySym::XK_emfilledrect => 0x0adf,
                XKeySym::XK_openstar => 0x0ae5,
                XKeySym::XK_leftpointer => 0x0aea,
                XKeySym::XK_rightpointer => 0x0aeb,
                XKeySym::XK_club => 0x0aec,
                XKeySym::XK_diamond => 0x0aed,
                XKeySym::XK_heart => 0x0aee,
                XKeySym::XK_maltesecross => 0x0af0,
                XKeySym::XK_dagger => 0x0af1,
                XKeySym::XK_doubledagger => 0x0af2,
                XKeySym::XK_checkmark => 0x0af3,
                XKeySym::XK_ballotcross => 0x0af4,
                XKeySym::XK_musicalsharp => 0x0af5,
                XKeySym::XK_musicalflat => 0x0af6,
                XKeySym::XK_malesymbol => 0x0af7,
                XKeySym::XK_femalesymbol => 0x0af8,
                XKeySym::XK_telephone => 0x0af9,
                XKeySym::XK_telephonerecorder => 0x0afa,
                XKeySym::XK_phonographcopyright => 0x0afb,
                XKeySym::XK_caret => 0x0afc,
                XKeySym::XK_singlelowquotemark => 0x0afd,
                XKeySym::XK_doublelowquotemark => 0x0afe,
                XKeySym::XK_cursor => 0x0aff,
                XKeySym::XK_leftcaret => 0x0ba3,
                XKeySym::XK_rightcaret => 0x0ba6,
                XKeySym::XK_downcaret => 0x0ba8,
                XKeySym::XK_upcaret => 0x0ba9,
                XKeySym::XK_overbar => 0x0bc0,
                XKeySym::XK_downtack => 0x0bc2,
                XKeySym::XK_upshoe => 0x0bc3,
                XKeySym::XK_downstile => 0x0bc4,
                XKeySym::XK_underbar => 0x0bc6,
                XKeySym::XK_jot => 0x0bca,
                XKeySym::XK_quad => 0x0bcc,
                XKeySym::XK_uptack => 0x0bce,
                XKeySym::XK_upstile => 0x0bd3,
                XKeySym::XK_downshoe => 0x0bd6,
                XKeySym::XK_rightshoe => 0x0bd8,
                XKeySym::XK_leftshoe => 0x0bda,
                XKeySym::XK_lefttack => 0x0bdc,
                XKeySym::XK_righttack => 0x0bfc,
                XKeySym::XK_Korean_Won => 0x0eff,
                XKeySym::XK_Ibreve => 0x100012c,
                XKeySym::XK_Zstroke => 0x10001b5,
                XKeySym::XK_Gcaron => 0x10001e6,
                XKeySym::XK_Ocaron => 0x10001d1,
                XKeySym::XK_Obarred => 0x100019f,
                XKeySym::XK_ibreve => 0x100012d,
                XKeySym::XK_zstroke => 0x10001b6,
                XKeySym::XK_gcaron => 0x10001e7,
                XKeySym::XK_ocaron => 0x10001d2,
                XKeySym::XK_obarred => 0x1000275,
                XKeySym::XK_SCHWA => 0x100018f,
                XKeySym::XK_schwa => 0x1000259,
                XKeySym::XK_EZH => 0x10001b7,
                XKeySym::XK_ezh => 0x1000292,
                XKeySym::XK_Abreveacute => 0x1001eae,
                XKeySym::XK_abreveacute => 0x1001eaf,
                XKeySym::XK_Abrevetilde => 0x1001eb4,
                XKeySym::XK_abrevetilde => 0x1001eb5,
                XKeySym::XK_Etilde => 0x1001ebc,
                XKeySym::XK_etilde => 0x1001ebd,
                XKeySym::XK_Ytilde => 0x1001ef8,
                XKeySym::XK_ytilde => 0x1001ef9,
                XKeySym::XK_EcuSign => 0x10020a0,
                XKeySym::XK_ColonSign => 0x10020a1,
                XKeySym::XK_CruzeiroSign => 0x10020a2,
                XKeySym::XK_FFrancSign => 0x10020a3,
                XKeySym::XK_LiraSign => 0x10020a4,
                XKeySym::XK_MillSign => 0x10020a5,
                XKeySym::XK_NairaSign => 0x10020a6,
                XKeySym::XK_PesetaSign => 0x10020a7,
                XKeySym::XK_RupeeSign => 0x10020a8,
                XKeySym::XK_WonSign => 0x10020a9,
                XKeySym::XK_NewSheqelSign => 0x10020aa,
                XKeySym::XK_DongSign => 0x10020ab,
                XKeySym::XK_EuroSign => 0x20ac,
                XKeySym::XF86XK_MonBrightnessUp => 0x1008FF02,
                XKeySym::XF86XK_MonBrightnessDown => 0x1008FF03,
                XKeySym::XF86XK_KbdLightOnOff => 0x1008FF04,
                XKeySym::XF86XK_KbdBrightnessUp => 0x1008FF05,
                XKeySym::XF86XK_KbdBrightnessDown => 0x1008FF06,
                XKeySym::XF86XK_MonBrightnessCycle => 0x1008FF07,
                XKeySym::XF86XK_Standby => 0x1008FF10,
                XKeySym::XF86XK_AudioLowerVolume => 0x1008FF11,
                XKeySym::XF86XK_AudioMute => 0x1008FF12,
                XKeySym::XF86XK_AudioRaiseVolume => 0x1008FF13,
                XKeySym::XF86XK_AudioPlay => 0x1008FF14,
                XKeySym::XF86XK_AudioStop => 0x1008FF15,
                XKeySym::XF86XK_AudioPrev => 0x1008FF16,
                XKeySym::XF86XK_AudioNext => 0x1008FF17,
            } as u32)
                .to_le_bytes()
                .to_vec()
                .into_iter()
                .filter(|&b| b > 0)
                .collect(),
        )?)
    }
}
