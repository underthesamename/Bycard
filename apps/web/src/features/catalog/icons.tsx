import {
  ArrowLeft,
  ArrowRight,
  BookOpen,
  ImageSquare,
  MagnifyingGlass,
  X,
  type IconProps,
} from "@phosphor-icons/react";

export function ArrowLeftIcon(props: IconProps) {
  return <ArrowLeft size={24} weight="regular" {...props} />;
}

export function ArrowRightIcon(props: IconProps) {
  return <ArrowRight size={24} weight="regular" {...props} />;
}

export function BookIcon(props: IconProps) {
  return <BookOpen size={24} weight="regular" {...props} />;
}

export function SearchIcon(props: IconProps) {
  return <MagnifyingGlass size={24} weight="regular" {...props} />;
}

export function ImageIcon(props: IconProps) {
  return <ImageSquare size={24} weight="regular" {...props} />;
}

export function CloseIcon(props: IconProps) {
  return <X size={24} weight="regular" {...props} />;
}
