unit types;
interface
type
  twordrec = record
    case byte of
      0 : (bytes : array[0..3] of byte);
      1 : (value : dword);
  end;
implementation
end.
