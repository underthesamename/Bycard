import { BinderScreen } from "../../../features/catalog/binder-screen";

type CollectionPageProps = Readonly<{
  params: Promise<{ setId: string }>;
}>;

export default async function CollectionPage({ params }: CollectionPageProps) {
  const { setId } = await params;
  return <BinderScreen setId={setId} />;
}
