unit cutils;
interface
function tostr(i : qword) : shortstring;    overload;
function tostr(i : int64) : shortstring;    overload;
function tostr(i : cardinal) : shortstring; overload;
function tostr(i : longint) : shortstring;  overload;
implementation
function tostr(i : qword) : shortstring;    begin tostr := ''; end;
function tostr(i : int64) : shortstring;    begin tostr := ''; end;
function tostr(i : cardinal) : shortstring; begin tostr := ''; end;
function tostr(i : longint) : shortstring;  begin tostr := ''; end;
end.
