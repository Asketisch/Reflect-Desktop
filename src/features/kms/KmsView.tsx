/**
 * KMS — Knowledge Management System view.
 *
 * Phase 3 item 12: grep-based wiki + /dream session mining.
 * Lists knowledge bases, allows creating/editing/searching pages.
 */
import { useState } from 'react';
import { BookOpen, Plus, Search, Trash2, FileText } from 'lucide-react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon, Button, EmptyState, Spinner, Input } from '@/features/design-system';
import {
  reflect_kms_list,
  reflect_kms_create,
  reflect_kms_delete,
  reflect_kms_list_pages,
  reflect_kms_save_page,
  reflect_kms_search,
  type ReflectWikiInfo,
  type ReflectPage,
  type ReflectSearchResult,
} from '@/utils/commands';
import s from './KmsView.module.css';

export function KmsView() {
  const queryClient = useQueryClient();
  const [selectedWiki, setSelectedWiki] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [newWikiName, setNewWikiName] = useState('');
  const [newWikiDesc, setNewWikiDesc] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [editingPage, setEditingPage] = useState<ReflectPage | null>(null);
  const [pageContent, setPageContent] = useState('');
  const [pageName, setPageName] = useState('');

  const wikisQuery = useQuery({
    queryKey: ['kms', 'wikis'],
    queryFn: reflect_kms_list,
  });

  const pagesQuery = useQuery({
    queryKey: ['kms', 'pages', selectedWiki],
    queryFn: () => reflect_kms_list_pages(selectedWiki!),
    enabled: !!selectedWiki,
  });

  const searchResults = useQuery({
    queryKey: ['kms', 'search', searchQuery],
    queryFn: () => reflect_kms_search(searchQuery),
    enabled: searchQuery.length > 1,
  });

  const createWiki = useMutation({
    mutationFn: () => reflect_kms_create(newWikiName, newWikiDesc || null),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['kms', 'wikis'] });
      setShowCreateForm(false);
      setNewWikiName('');
      setNewWikiDesc('');
    },
  });

  const deleteWiki = useMutation({
    mutationFn: (name: string) => reflect_kms_delete(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['kms', 'wikis'] });
      if (selectedWiki) setSelectedWiki(null);
    },
  });

  const savePage = useMutation({
    mutationFn: () =>
      reflect_kms_save_page(selectedWiki!, pageName, pageContent, null, null),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['kms', 'pages', selectedWiki] });
      setEditingPage(null);
      setPageContent('');
      setPageName('');
    },
  });

  return (
    <PageShell
      icon={BookOpen}
      title="Knowledge Base"
      subtitle="Grep-based wiki for local knowledge. Create, search, and manage pages."
      width="lg"
    >
      {/* Search bar */}
      <Card level="flat" padding="md" className={s.searchCard}>
        <div className={s.searchRow}>
          <Icon icon={Search} size={16} />
          <Input
            placeholder="Search across all knowledge bases..."
            value={searchQuery}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setSearchQuery(e.target.value)}
            className={s.searchInput}
          />
        </div>
        {searchResults.data && searchResults.data.length > 0 && (
          <div className={s.searchResults}>
            {searchResults.data.map((r: ReflectSearchResult, i: number) => (
              <div key={i} className={s.searchItem}>
                <Badge variant="info">{r.wiki}/{r.page}</Badge>
                <span className={s.snippet}>{r.lines[0]}</span>
              </div>
            ))}
          </div>
        )}
      </Card>

      {/* Wiki selector */}
      <div className={s.wikiBar}>
        {wikisQuery.data?.map((wiki: ReflectWikiInfo) => (
          <button
            key={wiki.name}
            type="button"
            className={s.wikiTab}
            data-active={selectedWiki === wiki.name || undefined}
            onClick={() => setSelectedWiki(wiki.name)}
          >
            {wiki.name}
            <Badge variant="neutral">{wiki.pageCount}</Badge>
          </button>
        ))}
        <Button variant="ghost" size="sm" onClick={() => setShowCreateForm(!showCreateForm)}>
          <Icon icon={Plus} size={14} /> New
        </Button>
      </div>

      {/* Create wiki form */}
      {showCreateForm && (
        <Card level="outlined" padding="md" className={s.createForm}>
          <Input
            placeholder="Wiki name (e.g. engineering)"
            value={newWikiName}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setNewWikiName(e.target.value)}
          />
          <Input
            placeholder="Description (optional)"
            value={newWikiDesc}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setNewWikiDesc(e.target.value)}
          />
          <Button
            variant="primary"
            size="sm"
            onClick={() => createWiki.mutate()}
            disabled={!newWikiName || createWiki.isPending}
          >
            Create
          </Button>
        </Card>
      )}

      {/* Selected wiki content */}
      {selectedWiki ? (
        <div className={s.wikiContent}>
          <div className={s.wikiHeader}>
            <h2 className={s.wikiTitle}>{selectedWiki}</h2>
            <div className={s.wikiActions}>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  setEditingPage(null);
                  setPageName('');
                  setPageContent('');
                }}
              >
                <Icon icon={Plus} size={14} /> New Page
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => deleteWiki.mutate(selectedWiki)}
              >
                <Icon icon={Trash2} size={14} /> Delete Wiki
              </Button>
            </div>
          </div>

          {editingPage !== null && (
            <Card level="outlined" padding="md" className={s.editorCard}>
              <Input
                placeholder="Page name (e.g. api-design)"
                value={pageName}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setPageName(e.target.value)}
              />
              <textarea
                className={s.editor}
                placeholder="Write your page content in Markdown..."
                value={pageContent}
                onChange={(e) => setPageContent(e.target.value)}
                rows={10}
              />
              <div className={s.editorActions}>
                <Button
                  variant="primary"
                  size="sm"
                  onClick={() => savePage.mutate()}
                  disabled={!pageName || savePage.isPending}
                >
                  Save
                </Button>
                <Button variant="ghost" size="sm" onClick={() => setEditingPage(null)}>
                  Cancel
                </Button>
              </div>
            </Card>
          )}

          {pagesQuery.isLoading ? (
            <Spinner size={20} />
          ) : pagesQuery.data?.length === 0 ? (
            <EmptyState
              icon={<Icon icon={FileText} />}
              title="No pages yet"
              description="Create a new page to start building your knowledge base."
            />
          ) : (
            <div className={s.pageList}>
              {pagesQuery.data?.map((page: ReflectPage) => (
                <Card key={page.name} level="outlined" padding="md" className={s.pageCard}>
                  <div className={s.pageHeader}>
                    <Icon icon={FileText} size={16} />
                    <span className={s.pageName}>{page.name}</span>
                    {page.tags.map((tag) => (
                      <Badge key={tag} variant="neutral">{tag}</Badge>
                    ))}
                  </div>
                  <pre className={s.pageContent}>{page.content.slice(0, 200)}</pre>
                </Card>
              ))}
            </div>
          )}
        </div>
      ) : wikisQuery.data?.length === 0 ? (
        <EmptyState
          icon={<Icon icon={BookOpen} />}
          title="No knowledge bases"
          description="Create your first wiki to start organizing knowledge."
        />
      ) : wikisQuery.isLoading ? (
        <Spinner size={20} />
      ) : null}
    </PageShell>
  );
}